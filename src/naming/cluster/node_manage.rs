use super::model::NamingRouteRequest;
use super::model::ProcessRange;
use super::model::SnapshotDataInfo;
use super::model::SnapshotForSend;
use super::model::SyncSenderRequest;
use super::model::{NamingRouteAddr, SyncSenderSetCmd};
use super::sync_sender::ClusteSyncSender;
use crate::naming::core::NamingResult;
use crate::naming::model::{DistroData, InstanceKey};
use crate::{
    naming::core::{NamingActor, NamingCmd},
    now_millis, now_second_i32,
    raft::network::factory::RaftClusterRequestSender,
};
use actix::prelude::*;
use bean_factory::{bean, Inject};
use std::collections::HashMap;
use std::{
    collections::{hash_map::DefaultHasher, BTreeMap, HashSet},
    hash::{Hash, Hasher},
    sync::Arc,
    time::Duration,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeStatus {
    Valid,
    Invalid,
}

impl Default for NodeStatus {
    fn default() -> Self {
        Self::Valid
    }
}

#[derive(Default, Debug, Clone)]
pub struct ClusterNode {
    pub id: u64,
    pub index: u64,
    pub is_local: bool,
    pub addr: Arc<String>,
    pub status: NodeStatus,
}

#[derive(Default, Debug, Clone)]
pub struct ClusterInnerNode {
    pub id: u64,
    pub index: u64,
    pub is_local: bool,
    pub addr: Arc<String>,
    pub status: NodeStatus,
    pub last_active_time: u64,
    pub sync_sender: Option<Addr<ClusteSyncSender>>,
    pub client_set: HashSet<Arc<String>>,
}

impl ClusterInnerNode {
    pub(crate) fn is_valid(&self) -> bool {
        self.is_local || self.status == NodeStatus::Valid
    }
}

impl From<ClusterInnerNode> for ClusterNode {
    fn from(value: ClusterInnerNode) -> Self {
        Self {
            id: value.id,
            index: value.index,
            is_local: value.is_local,
            addr: value.addr,
            status: value.status,
        }
    }
}

#[bean(inject)]
pub struct InnerNodeManage {
    local_id: u64,
    all_nodes: BTreeMap<u64, ClusterInnerNode>,
    cluster_sender: Option<Arc<RaftClusterRequestSender>>,
    naming_actor: Option<Addr<NamingActor>>,
    first_query_snapshot: bool,
    current_range: ProcessRange,
    history_ranges: Vec<(ProcessRange, u64)>,
    last_send_distor_data_time: i32,
}

impl InnerNodeManage {
    pub fn new(local_id: u64) -> Self {
        Self {
            local_id,
            cluster_sender: None,
            all_nodes: Default::default(),
            naming_actor: None,
            first_query_snapshot: false,
            current_range: ProcessRange { index: 0, len: 1 },
            history_ranges: Vec::new(),
            last_send_distor_data_time: 0,
        }
    }

    fn update_nodes(&mut self, nodes: Vec<(u64, Arc<String>)>, ctx: &mut Context<Self>) {
        if self.cluster_sender.is_none() {
            log::warn!("InnerNodeManage cluster_sender is none");
            return;
        }
        let new_sets: HashSet<u64> = nodes.iter().map(|e| e.0.to_owned()).collect();
        let mut dels = vec![];
        for key in self.all_nodes.keys() {
            if !new_sets.contains(key) {
                dels.push(*key);
            }
        }
        let mut is_change = !dels.is_empty();
        for key in dels {
            self.all_nodes.remove(&key);
        }
        let now = now_millis();
        for (key, addr) in nodes {
            if let Some(node) = self.all_nodes.get_mut(&key) {
                if let Some(sender) = node.sync_sender.as_ref() {
                    sender.do_send(SyncSenderSetCmd::UpdateTargetAddr(addr.clone()));
                };
                node.addr = addr;
            } else {
                let is_local = self.local_id == key;
                let sync_sender = if is_local {
                    None
                } else {
                    is_change = true;
                    Some(
                        ClusteSyncSender::new(
                            self.local_id,
                            key,
                            addr.clone(),
                            self.cluster_sender.clone(),
                        )
                        .start(),
                    )
                };
                let node = ClusterInnerNode {
                    id: key,
                    index: 0,
                    is_local,
                    addr,
                    sync_sender,
                    status: NodeStatus::Valid,
                    last_active_time: now,
                    client_set: Default::default(),
                };
                self.all_nodes.insert(key, node);
            }
        }
        let local_node = self.get_this_node();
        self.all_nodes.entry(self.local_id).or_insert(local_node);
        self.update_nodes_index();
        self.update_process_range();
        self.first_query_snapshot(ctx);
        if is_change {
            //集群节点变更化重新刷新服务管理范围
            self.refresh_process_range();
        }
    }

    fn first_query_snapshot(&mut self, ctx: &mut Context<InnerNodeManage>) {
        //第一次需要触发从其它实例加载snapshot
        if !self.first_query_snapshot {
            self.first_query_snapshot = true;
            //从其它节点同步数据
            //1秒
            ctx.run_later(Duration::from_millis(1000), |act, _ctx| {
                act.load_snapshot_from_node();
            });
            //15秒
            ctx.run_later(Duration::from_millis(15000), |act, _ctx| {
                act.load_snapshot_from_node();
            });
            //45秒
            ctx.run_later(Duration::from_millis(45_000), |act, _ctx| {
                act.load_snapshot_from_node();
            });
            //因涉负责区域动荡，要把加入后的节点数据也同步给其它节点
            //30秒
            ctx.run_later(Duration::from_millis(30_000), |act, ctx| {
                act.notify_snapshot_to_node(ctx);
            });
        }
    }

    fn refresh_process_range(&mut self) {
        if let Some(naming_actor) = &self.naming_actor {
            naming_actor.do_send(NamingCmd::ClusterRefreshProcessRange(
                self.current_range.clone(),
            ));
        }
    }

    fn update_nodes_index(&mut self) {
        for (i, value) in self.all_nodes.values_mut().enumerate() {
            value.index = i as u64;
        }
    }

    fn get_this_node(&self) -> ClusterInnerNode {
        if let Some(node) = self.all_nodes.get(&self.local_id) {
            node.to_owned()
        } else {
            ClusterInnerNode {
                id: self.local_id,
                is_local: true,
                ..Default::default()
            }
        }
    }

    fn get_all_nodes(&self) -> Vec<ClusterNode> {
        if self.all_nodes.is_empty() {
            vec![self.get_this_node().into()]
        } else {
            self.all_nodes.values().cloned().map(|e| e.into()).collect()
        }
    }

    fn update_process_range(&mut self) {
        let new_range = self.get_current_process_range();
        if new_range == self.current_range {
            return;
        }
        self.clear_timeout_process_range();
        self.history_ranges
            .push((self.current_range.clone(), now_millis()));
        self.current_range = new_range;
    }

    fn clear_timeout_process_range(&mut self) {
        let timeout = now_millis() - 60_000_000;
        let mut new_list = vec![];
        for (range, t) in &self.history_ranges {
            if timeout < *t {
                new_list.push((range.to_owned(), *t))
            }
        }
        self.history_ranges = new_list;
    }

    fn get_current_process_range(&self) -> ProcessRange {
        if self.all_nodes.is_empty() {
            ProcessRange::new(0, 1)
        } else {
            ProcessRange::new(
                self.get_this_node().index as usize,
                self.all_nodes.iter().filter(|(_, v)| v.is_valid()).count(),
            )
        }
    }

    fn get_cluster_process_range(&mut self, _input: ProcessRange) -> Vec<ProcessRange> {
        let mut list = vec![];
        list.push(self.current_range.clone());
        self.clear_timeout_process_range();
        for (r, _) in &self.history_ranges {
            if *r != self.current_range {
                list.push(r.to_owned());
            }
        }
        list
    }

    fn send_to_other_node(&self, req: SyncSenderRequest, is_valid: bool) {
        for node in self.all_nodes.values() {
            if node.is_local || (is_valid && node.status != NodeStatus::Valid) {
                continue;
            }
            if let Some(sync_sender) = node.sync_sender.as_ref() {
                sync_sender.do_send(req.clone());
            }
        }
    }

    fn load_snapshot_from_node(&self) {
        let list: Vec<(&u64, &ClusterInnerNode)> = self
            .all_nodes
            .iter()
            .filter(|(_k, e)| e.is_valid())
            .collect();
        let len = list.len();
        for (_, node) in list {
            if node.is_local {
                continue;
            };
            let req = SyncSenderRequest(NamingRouteRequest::QuerySnapshot {
                index: node.index as usize,
                len,
            });
            if let Some(sync_sender) = node.sync_sender.as_ref() {
                sync_sender.do_send(req.clone());
            }
        }
    }

    fn notify_snapshot_to_node(&mut self, ctx: &mut Context<Self>) {
        if self.naming_actor.is_none() || self.all_nodes.len() <= 1 {
            return;
        }
        let naming_addr = self.naming_actor.clone().unwrap();
        let ranges = vec![self.current_range.clone()];
        Self::query_snapshot_data(naming_addr, ranges)
            .into_actor(self)
            .map(|res, act, _ctx| {
                if let Ok(req) = res {
                    act.send_to_other_node(req, false);
                }
            })
            .spawn(ctx);
    }

    async fn query_snapshot_data(
        naming_addr: Addr<NamingActor>,
        ranges: Vec<ProcessRange>,
    ) -> anyhow::Result<SyncSenderRequest> {
        let result = naming_addr.send(NamingCmd::QuerySnapshot(ranges)).await??;
        if let NamingResult::Snapshot(snapshot) = result {
            let snapshot = SnapshotDataInfo::from(snapshot);
            let data = snapshot.to_bytes()?;
            Ok(SyncSenderRequest(NamingRouteRequest::Snapshot(data)))
        } else {
            Err(anyhow::anyhow!("query snapshot error"))
        }
    }

    fn send_snapshot_to_node(&self, node_id: u64, snapshot_for_send: SnapshotForSend) {
        if let Some(n) = self.all_nodes.get(&node_id) {
            let snapshot = SnapshotDataInfo::from(snapshot_for_send);
            let data = snapshot.to_bytes();
            if let (Ok(data), Some(sender)) = (data, &n.sync_sender) {
                sender.do_send(SyncSenderRequest(NamingRouteRequest::Snapshot(data)));
            }
        }
    }

    fn send_diff_instance_to_node(&self, node_id: u64, diff_instance: Vec<InstanceKey>) {
        if let Some(n) = self.all_nodes.get(&node_id) {
            if let Some(sender) = n.sync_sender.as_ref() {
                sender.do_send(SyncSenderRequest(
                    NamingRouteRequest::QueryDistroInstanceSnapshot(diff_instance),
                ));
            }
        }
    }

    fn check_node_status(&mut self) {
        let timeout = now_millis() - 15000;
        let naming_actor = &self.naming_actor;
        for node in self.all_nodes.values_mut() {
            /*
            //log for debug
            log::warn!("NAMING_NODE_CHECK node:{} status:{} client_set:{} timeout:{}",
                node.id,
                node.is_valid(),
                &serde_json::to_string(&node.client_set).unwrap_or_default(),
                node.last_active_time < timeout
            );
            */
            if !node.is_local && node.status == NodeStatus::Valid && node.last_active_time < timeout
            {
                node.status = NodeStatus::Invalid;
                Self::client_invalid_instance(naming_actor, node);
            }
        }
        self.update_process_range();
    }

    fn client_invalid_instance(
        naming_actor: &Option<Addr<NamingActor>>,
        node: &mut ClusterInnerNode,
    ) {
        if let Some(naming_actor) = naming_actor.as_ref() {
            for client_id in &node.client_set {
                naming_actor.do_send(NamingCmd::RemoveClientFromCluster(client_id.clone()));
            }
        }
        node.client_set.clear();
    }

    fn ping_other(&mut self) {
        let req = SyncSenderRequest(NamingRouteRequest::Ping(self.local_id));
        self.send_to_other_node(req, false);
    }

    fn send_distort_data(&mut self, ctx: &mut Context<Self>) {
        let now_second = now_second_i32();
        //间隔同步
        if now_second - self.last_send_distor_data_time < 12 {
            return;
        }
        self.last_send_distor_data_time = now_second;
        if self.all_nodes.len() < 2 || self.naming_actor.is_none() {
            return;
        }
        let naming_addr = self.naming_actor.as_ref().unwrap().clone();
        Self::load_send_distort_data(naming_addr)
            .into_actor(self)
            .map(|res, act, _ctx| {
                if let Ok(data) = res {
                    let instance_count: usize = data.values().map(|keys| keys.len()).sum();
                    log::info!(
                        "send_distort_data,client_count:{},instance_count:{}",
                        data.len(),
                        instance_count
                    );
                    for node in act.all_nodes.values_mut() {
                        if node.is_local {
                            continue;
                        }
                        if let Some(sender) = &node.sync_sender {
                            sender.do_send(SyncSenderRequest(
                                NamingRouteRequest::SyncDistroClientInstances(data.clone()),
                            ));
                        }
                    }
                }
            })
            .spawn(ctx);
    }

    async fn load_send_distort_data(
        naming_addr: Addr<NamingActor>,
    ) -> anyhow::Result<HashMap<Arc<String>, HashSet<InstanceKey>>> {
        if let NamingResult::GrpcDistroData(DistroData::ClientInstances(client_data)) =
            naming_addr.send(NamingCmd::QueryGrpcDistroData).await??
        {
            Ok(client_data)
        } else {
            Ok(HashMap::new())
        }
    }

    fn hb(&mut self, ctx: &mut Context<Self>) {
        ctx.run_later(Duration::from_millis(3000), |act, ctx| {
            act.check_node_status();
            act.ping_other();
            act.send_distort_data(ctx);
            act.hb(ctx);
        });
    }

    fn active_node(&mut self, node_id: u64) {
        if let Some(node) = self.all_nodes.get_mut(&node_id) {
            node.last_active_time = now_millis();
            node.status = NodeStatus::Valid;
        }
    }

    fn remove_client_id(&mut self, client_id: Arc<String>) {
        for node in self.all_nodes.values_mut() {
            node.client_set.remove(&client_id);
        }
        if let Some(naming_actor) = self.naming_actor.as_ref() {
            naming_actor.do_send(NamingCmd::RemoveClientFromCluster(client_id));
        }
    }

    fn node_add_client(&mut self, node_id: u64, client_id: Arc<String>) {
        if client_id.is_empty() {
            return;
        }
        if let Some(node) = self.all_nodes.get_mut(&node_id) {
            node.last_active_time = now_millis();
            node.status = NodeStatus::Valid;
            node.client_set.insert(client_id);
        }
    }

    fn node_diff_clients(&mut self, node_id: u64, client_ids: HashSet<Arc<String>>) {
        let mut remove_client_ids = vec![];
        if let Some(node) = self.all_nodes.get_mut(&node_id) {
            remove_client_ids = node.client_set.difference(&client_ids).cloned().collect();
            for item in &remove_client_ids {
                node.client_set.remove(item);
            }
        }
        if let Some(naming_actor) = self.naming_actor.as_ref() {
            naming_actor.do_send(NamingCmd::RemoveClientsFromCluster(remove_client_ids));
        }
    }
}

impl Actor for InnerNodeManage {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        log::info!("InnerNodeManage started!");

        //定时检测节点的可用性
        self.hb(ctx);
    }
}

impl Inject for InnerNodeManage {
    type Context = Context<Self>;

    fn inject(
        &mut self,
        factory_data: bean_factory::FactoryData,
        _factory: bean_factory::BeanFactory,
        _ctx: &mut Self::Context,
    ) {
        self.naming_actor = factory_data.get_actor();
        self.cluster_sender = factory_data.get_bean();
        log::info!("InnerNodeManage inject complete!");
    }
}

#[derive(Message, Debug)]
#[rtype(result = "anyhow::Result<NodeManageResponse>")]
pub enum NodeManageRequest {
    UpdateNodes(Vec<(u64, Arc<String>)>),
    GetThisNode,
    GetAllNodes,
    GetNode(u64),
    ActiveNode(u64),
    SendToOtherNodes(NamingRouteRequest),
    AddClientId(u64, Arc<String>),
    AddClientIds(u64, HashSet<Arc<String>>),
    RemoveDiffClientIds(u64, HashSet<Arc<String>>),
    RemoveClientId(Arc<String>),
    //QueryClusterIds(),
    QueryOwnerRange(ProcessRange),
    SendSnapshot(u64, SnapshotForSend),
    QueryDiffClientInstances(u64, Vec<InstanceKey>),
}

pub enum NodeManageResponse {
    None,
    ThisNode(ClusterNode),
    Node(Option<ClusterNode>),
    AllNodes(Vec<ClusterNode>),
    OwnerRange(Vec<ProcessRange>),
    RemoveClientIds(HashSet<Arc<String>>),
}

impl Handler<NodeManageRequest> for InnerNodeManage {
    type Result = anyhow::Result<NodeManageResponse>;

    fn handle(&mut self, msg: NodeManageRequest, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            NodeManageRequest::UpdateNodes(nodes) => {
                log::info!("InnerNodeManage UpdateNodes,size:{}", nodes.len());
                self.update_nodes(nodes, ctx);
                Ok(NodeManageResponse::None)
            }
            NodeManageRequest::GetThisNode => {
                Ok(NodeManageResponse::ThisNode(self.get_this_node().into()))
            }
            NodeManageRequest::GetNode(node_id) => {
                let node = self.all_nodes.get(&node_id).map(|e| e.to_owned().into());
                Ok(NodeManageResponse::Node(node))
            }
            NodeManageRequest::GetAllNodes => {
                Ok(NodeManageResponse::AllNodes(self.get_all_nodes()))
            }
            NodeManageRequest::SendToOtherNodes(req) => {
                self.send_to_other_node(SyncSenderRequest(req), true);
                Ok(NodeManageResponse::None)
            }
            NodeManageRequest::ActiveNode(node_id) => {
                self.active_node(node_id);
                Ok(NodeManageResponse::None)
            }
            NodeManageRequest::AddClientId(node_id, client_id) => {
                self.active_node(node_id);
                self.node_add_client(node_id, client_id);
                Ok(NodeManageResponse::None)
            }
            NodeManageRequest::AddClientIds(node_id, client_id_set) => {
                self.active_node(node_id);
                for client_id in client_id_set {
                    self.node_add_client(node_id, client_id);
                }
                Ok(NodeManageResponse::None)
            }
            NodeManageRequest::RemoveDiffClientIds(node_id, client_id_set) => {
                self.node_diff_clients(node_id, client_id_set);
                Ok(NodeManageResponse::None)
            }
            NodeManageRequest::RemoveClientId(client_id) => {
                self.remove_client_id(client_id);
                Ok(NodeManageResponse::None)
            }
            NodeManageRequest::QueryOwnerRange(range) => {
                let ranges = self.get_cluster_process_range(range);
                Ok(NodeManageResponse::OwnerRange(ranges))
            }
            NodeManageRequest::SendSnapshot(node_id, snapshot) => {
                self.send_snapshot_to_node(node_id, snapshot);
                Ok(NodeManageResponse::None)
            }
            NodeManageRequest::QueryDiffClientInstances(node_id, diff_instances) => {
                self.send_diff_instance_to_node(node_id, diff_instances);
                Ok(NodeManageResponse::None)
            }
        }
    }
}

#[derive(Debug)]
pub struct NodeManage {
    pub(crate) inner_node_manage: Addr<InnerNodeManage>,
}

impl NodeManage {
    pub fn new(inner_node_manage: Addr<InnerNodeManage>) -> Self {
        Self { inner_node_manage }
    }

    pub async fn route_addr<T: Hash>(&self, v: &T) -> NamingRouteAddr {
        let mut hasher = DefaultHasher::new();
        v.hash(&mut hasher);
        let hash_value: usize = hasher.finish() as usize;
        let nodes = self.get_all_valid_nodes().await.unwrap_or_default();
        if nodes.is_empty() {
            NamingRouteAddr::Local(0)
        } else {
            let index = hash_value % nodes.len();
            let node = nodes.get(index).unwrap();
            if node.is_local {
                NamingRouteAddr::Local(index as u64)
            } else {
                NamingRouteAddr::Remote(index as u64, node.addr.clone())
            }
        }
    }

    pub async fn get_node_addr(&self, node_id: u64) -> anyhow::Result<Arc<String>> {
        let resp: NodeManageResponse = self
            .inner_node_manage
            .send(NodeManageRequest::GetNode(node_id))
            .await??;
        match resp {
            NodeManageResponse::Node(node) => {
                if let Some(node) = node {
                    Ok(node.addr)
                } else {
                    Err(anyhow::anyhow!("the node {} addr is empty", &node_id))
                }
            }
            _ => Err(anyhow::anyhow!("get_node_addr error NodeManageResponse!")),
        }
    }

    pub async fn get_all_valid_nodes(&self) -> anyhow::Result<Vec<ClusterNode>> {
        let resp: NodeManageResponse = self
            .inner_node_manage
            .send(NodeManageRequest::GetAllNodes)
            .await??;
        match resp {
            NodeManageResponse::AllNodes(nodes) => Ok(nodes
                .into_iter()
                .filter(|e| e.status == NodeStatus::Valid)
                .collect()),
            _ => Err(anyhow::anyhow!(
                "get_all_valid_nodes error NodeManageResponse!"
            )),
        }
    }
    pub async fn get_other_valid_nodes(&self) -> anyhow::Result<Vec<ClusterNode>> {
        Ok(self
            .get_all_valid_nodes()
            .await?
            .into_iter()
            .filter(|e| !e.is_local)
            .collect())
    }

    pub fn active_node(&self, node_id: u64) {
        self.inner_node_manage
            .do_send(NodeManageRequest::ActiveNode(node_id))
    }
}
