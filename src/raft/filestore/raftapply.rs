#![allow(clippy::single_match)]
use std::sync::Arc;

use super::{
    log::SnapshotRange,
    model::{ApplyRequestDto, LogRecordLoader, MemberShip, SnapshotHeaderDto},
    raftindex::{RaftIndexManager, RaftIndexRequest, RaftIndexResponse},
    raftlog::{RaftLogManager, RaftLogManagerAsyncRequest, RaftLogManagerRequest},
    raftsnapshot::{
        RaftSnapshotManager, RaftSnapshotRequest, RaftSnapshotResponse, SnapshotReader,
    },
    StoreUtils,
};
use crate::common::byte_utils::bin_to_id;
use crate::common::constant::{
    CACHE_TREE_NAME, CONFIG_TREE_NAME, NAMESPACE_TREE_NAME, SEQUENCE_TREE_NAME, SEQ_KEY_CONFIG,
    USER_TREE_NAME,
};
use crate::config::core::{ConfigCmd, ConfigKey};
use crate::config::model::{ConfigRaftCmd, ConfigValueDO};
use crate::namespace::model::{NamespaceParam, NamespaceRaftReq};
use crate::raft::db::table::{TableManagerInnerReq, TableManagerReq};
use crate::raft::filestore::model::SnapshotRecordDto;
use crate::raft::filestore::raftdata::RaftDataWrap;
use crate::raft::filestore::raftsnapshot::SnapshotWriterActor;
use crate::raft::store::{ClientRequest, ClientResponse};
use actix::prelude::*;
use async_raft::raft::EntryPayload;
use async_raft_ext as async_raft;
use async_trait::async_trait;
use bean_factory::{bean, Inject};

pub struct LogRecordLoaderInstance {
    pub(crate) data_wrap: Arc<RaftDataWrap>,
    pub(crate) index_manager: Addr<RaftIndexManager>,
}

impl LogRecordLoaderInstance {
    fn new(data_wrap: Arc<RaftDataWrap>, index_manager: Addr<RaftIndexManager>) -> Self {
        Self {
            data_wrap,
            index_manager,
        }
    }
}

#[async_trait]
impl LogRecordLoader for LogRecordLoaderInstance {
    async fn load(&self, record: super::model::LogRecordDto) -> anyhow::Result<()> {
        let entry = StoreUtils::log_record_to_entry(record)?;
        match entry.payload {
            EntryPayload::Normal(req) => match req.data {
                ClientRequest::NodeAddr { id, addr } => {
                    self.index_manager
                        .send(RaftIndexRequest::AddNodeAddr(id, addr))
                        .await
                        .ok();
                }
                ClientRequest::Members(member) => {
                    self.index_manager
                        .send(RaftIndexRequest::SaveMember {
                            member: member.clone(),
                            member_after_consensus: None,
                            node_addr: None,
                        })
                        .await
                        .ok();
                }
                ClientRequest::ConfigSet {
                    key,
                    value,
                    config_type,
                    desc,
                    history_id,
                    history_table_id,
                    op_time,
                    op_user,
                } => {
                    let cmd = ConfigRaftCmd::ConfigAdd {
                        key,
                        value,
                        config_type,
                        desc,
                        history_id,
                        history_table_id,
                        op_time,
                        op_user,
                    };
                    self.data_wrap.config.send(cmd).await.ok();
                }
                ClientRequest::ConfigRemove { key } => {
                    let cmd = ConfigRaftCmd::ConfigRemove { key };
                    self.data_wrap.config.send(cmd).await.ok();
                }
                ClientRequest::TableManagerReq(req) => {
                    self.data_wrap.table.send(req).await.ok();
                }
                ClientRequest::NamespaceReq(req) => {
                    self.data_wrap.namespace.send(req).await.ok();
                }
            },
            _ => {}
        }
        Ok(())
    }
}

#[bean(inject)]
pub struct StateApplyManager {
    index_manager: Option<Addr<RaftIndexManager>>,
    snapshot_manager: Option<Addr<RaftSnapshotManager>>,
    log_manager: Option<Addr<RaftLogManager>>,
    data_wrap: Option<Arc<RaftDataWrap>>,
    snapshot_next_index: u64,
    last_applied_log: u64,
}

impl Default for StateApplyManager {
    fn default() -> Self {
        Self::new()
    }
}

impl StateApplyManager {
    pub fn new() -> Self {
        Self {
            index_manager: None,
            snapshot_manager: None,
            log_manager: None,
            data_wrap: None,
            snapshot_next_index: 1,
            last_applied_log: 0,
        }
    }

    fn init(&mut self, ctx: &mut Context<Self>) {
        self.load_index(ctx);
        //加载历史数据
    }

    fn load_index(&mut self, ctx: &mut Context<Self>) {
        if self.index_manager.is_none() {
            return;
        }
        let index_manager = self.index_manager.clone().unwrap();
        async move {
            index_manager
                .send(super::raftindex::RaftIndexRequest::LoadIndexInfo)
                .await?
        }
        .into_actor(self)
        .map(|r, act, ctx| {
            if let Ok(RaftIndexResponse::RaftIndexInfo {
                raft_index,
                last_applied_log,
            }) = r
            {
                log::info!("load_index,{:?}", &raft_index);
                if let Some(e) = raft_index.snapshots.last() {
                    act.snapshot_next_index = e.end_index + 1;
                }
                act.last_applied_log = last_applied_log;
            }
            //加载镜像,镜像转成状态
            act.load_snapshot(ctx);
        })
        .wait(ctx);
    }

    fn load_snapshot(&mut self, ctx: &mut Context<Self>) {
        if self.snapshot_next_index == 0 || self.snapshot_manager.is_none()
        //|| self.data_store.is_none()
        {
            self.load_log(ctx);
            return;
        }
        let snapshot_manager = self.snapshot_manager.clone().unwrap();
        //let data_store = self.data_store.clone().unwrap();
        let data_wrap = self.data_wrap.clone().unwrap();
        async move {
            if let RaftSnapshotResponse::LastSnapshot(Some(path), _) = snapshot_manager
                .send(RaftSnapshotRequest::GetLastSnapshot)
                .await??
            {
                let reader = SnapshotReader::init(&path).await?;
                log::info!("load_snapshot header,{:?}", &reader.get_header());
                Self::do_load_snapshot(data_wrap, reader).await?;
            }
            Ok(())
        }
        .into_actor(self)
        .map(|_r: anyhow::Result<()>, act, ctx| {
            //加载日志,日志转成状态
            act.load_log(ctx);
        })
        .wait(ctx);
    }

    fn apply_snapshot(&mut self, ctx: &mut Context<Self>, file: Box<tokio::fs::File>) {
        /*
        if self.data_store.is_none() && self.index_manager.is_none() {
            return;
        }
        let data_store = self.data_store.clone().unwrap();
         */
        let index_manager = self.index_manager.clone().unwrap();
        async move {
            let reader = SnapshotReader::init_by_file(file).await?;
            let header = reader.get_header();
            let member_after_consensus = if header.member_after_consensus.is_empty() {
                None
            } else {
                Some(header.member_after_consensus.clone())
            };
            index_manager.do_send(RaftIndexRequest::SaveMember {
                member: header.member.clone(),
                member_after_consensus,
                node_addr: Some(header.node_addrs.clone()),
            });
            //Self::do_load_snapshot(reader).await?;

            Ok(())
        }
        .into_actor(self)
        .map(|_r: anyhow::Result<()>, _act, _ctx| {})
        .wait(ctx);
    }

    async fn do_load_snapshot(
        data_wrap: Arc<RaftDataWrap>,
        mut reader: SnapshotReader,
    ) -> anyhow::Result<()> {
        while let Ok(Some(record)) = reader.read_record().await {
            if record.tree.as_str() == CONFIG_TREE_NAME.as_str() {
                let config_key = ConfigKey::from(&String::from_utf8(record.key)? as &str);
                let value_do = ConfigValueDO::from_bytes(&record.value)?;
                data_wrap
                    .config
                    .send(ConfigCmd::InnerSet(config_key, value_do.into()))
                    .await??;
            } else if record.tree.as_str() == SEQUENCE_TREE_NAME.as_str() {
                let key = String::from_utf8(record.key)?;
                let last_id = bin_to_id(&record.value);
                if &key as &str == SEQ_KEY_CONFIG {
                    data_wrap
                        .config
                        .send(ConfigCmd::InnerSetLastId(last_id))
                        .await??;
                };
            } else if record.tree.as_str() == USER_TREE_NAME.as_str() {
                let key = record.key;
                let value = record.value;
                let req = TableManagerReq::Set {
                    table_name: USER_TREE_NAME.clone(),
                    key,
                    value,
                    last_seq_id: None,
                };
                data_wrap.table.send(req).await??;
            } else if record.tree.as_str() == CACHE_TREE_NAME.as_str() {
                let key = record.key;
                let value = record.value;
                let req = TableManagerReq::Set {
                    table_name: CACHE_TREE_NAME.clone(),
                    key,
                    value,
                    last_seq_id: None,
                };
                data_wrap.table.send(req).await??;
            } else if record.tree.as_str() == NAMESPACE_TREE_NAME.as_str() {
                let req = RaftApplyDataRequest::LoadSnapshotRecord(record);
                data_wrap.namespace.send(req).await??;
            } else {
                log::warn!(
                    "do_load_snapshot ignore data,table name:{}",
                    record.tree.as_str()
                );
            }
        }
        Ok(())
    }

    fn load_log(&mut self, ctx: &mut Context<Self>) {
        if self.last_applied_log == 0 || self.log_manager.is_none() || self.data_wrap.is_none() {
            return;
        }
        let start_index = self.snapshot_next_index;
        let end_index = self.last_applied_log + 1;
        let log_manager = self.log_manager.clone().unwrap();
        let index_manager = self.index_manager.clone().unwrap();
        let data_wrap = self.data_wrap.clone().unwrap();
        //let data_store = self.data_store.clone().unwrap();
        let loader = Arc::new(LogRecordLoaderInstance::new(data_wrap, index_manager));
        async move {
            log_manager
                .send(RaftLogManagerAsyncRequest::Load {
                    start: start_index,
                    end: end_index,
                    loader,
                })
                .await??;
            Ok(())
        }
        .into_actor(self)
        .map(|_r: anyhow::Result<()>, act, ctx| {
            act.load_complete(ctx);
        })
        .wait(ctx);
    }

    /// raft数据加载完成通知
    fn load_complete(&mut self, _ctx: &mut Context<Self>) {
        log::info!("raft data load finished.");
        if let Some(data_wrap) = self.data_wrap.as_ref() {
            data_wrap
                .namespace
                .do_send(RaftApplyDataRequest::LoadCompleted);
        }
    }

    fn apply_request_to_state_machine(&mut self, request: ApplyRequestDto) -> anyhow::Result<()> {
        //self.last_applied_log = request.index;
        //todo
        match request.request {
            ClientRequest::NodeAddr { id, addr } => {
                if let Some(index_manager) = &self.index_manager {
                    index_manager.do_send(RaftIndexRequest::AddNodeAddr(id, addr));
                }
            }
            ClientRequest::Members(member) => {
                if let Some(index_manager) = &self.index_manager {
                    index_manager.do_send(RaftIndexRequest::SaveMember {
                        member: member.clone(),
                        member_after_consensus: None,
                        node_addr: None,
                    });
                }
            }
            ClientRequest::ConfigSet {
                key,
                value,
                config_type,
                desc,
                history_id,
                history_table_id,
                op_time,
                op_user,
            } => {
                if let Some(raft_data_wrap) = &self.data_wrap {
                    let cmd = ConfigRaftCmd::ConfigAdd {
                        key,
                        value,
                        config_type,
                        desc,
                        history_id,
                        history_table_id,
                        op_time,
                        op_user,
                    };
                    raft_data_wrap.config.do_send(cmd);
                }
            }
            ClientRequest::ConfigRemove { key } => {
                if let Some(raft_data_wrap) = &self.data_wrap {
                    let cmd = ConfigRaftCmd::ConfigRemove { key };
                    raft_data_wrap.config.do_send(cmd);
                }
            }
            ClientRequest::TableManagerReq(req) => {
                if let Some(raft_data_wrap) = &self.data_wrap {
                    raft_data_wrap.table.do_send(req);
                }
            }
            ClientRequest::NamespaceReq(req) => {
                if let Some(raft_data_wrap) = &self.data_wrap {
                    raft_data_wrap.namespace.do_send(req);
                }
            }
        };
        Ok(())
    }

    async fn async_apply_request_to_state_machine(
        request: ApplyRequestDto,
        raft_data_wrap: &RaftDataWrap,
        index_manager: Addr<RaftIndexManager>,
    ) -> anyhow::Result<ClientResponse> {
        let last_applied_log = request.index;
        let r = match request.request {
            ClientRequest::NodeAddr { id, addr } => {
                index_manager.do_send(RaftIndexRequest::AddNodeAddr(id, addr));
                Ok(ClientResponse::Success)
            }
            ClientRequest::Members(member) => {
                index_manager.do_send(RaftIndexRequest::SaveMember {
                    member: member.clone(),
                    member_after_consensus: None,
                    node_addr: None,
                });
                Ok(ClientResponse::Success)
            }
            ClientRequest::ConfigSet {
                key,
                value,
                config_type,
                desc,
                history_id,
                history_table_id,
                op_time,
                op_user,
            } => {
                let cmd = ConfigRaftCmd::ConfigAdd {
                    key,
                    value,
                    config_type,
                    desc,
                    history_id,
                    history_table_id,
                    op_time,
                    op_user,
                };
                raft_data_wrap.config.send(cmd).await??;
                Ok(ClientResponse::Success)
            }
            ClientRequest::ConfigRemove { key } => {
                let cmd = ConfigRaftCmd::ConfigRemove { key };
                raft_data_wrap.config.send(cmd).await??;
                Ok(ClientResponse::Success)
            }
            ClientRequest::TableManagerReq(req) => {
                raft_data_wrap.table.send(req).await??;
                Ok(ClientResponse::Success)
            }
            ClientRequest::NamespaceReq(req) => {
                raft_data_wrap.namespace.send(req).await??;
                Ok(ClientResponse::Success)
            }
        };
        index_manager.do_send(RaftIndexRequest::SaveLastAppliedLog(last_applied_log));
        r
    }

    async fn do_build_snapshot(
        log_manager: Addr<RaftLogManager>,
        index_manager: Addr<RaftIndexManager>,
        snapshot_manager: Addr<RaftSnapshotManager>,
        data_wrap: Arc<RaftDataWrap>,
        last_index: u64,
    ) -> anyhow::Result<(SnapshotHeaderDto, Arc<String>, u64)> {
        //1. get last applied log
        let last_log = match log_manager
            .send(RaftLogManagerAsyncRequest::Query {
                start: last_index,
                end: last_index + 1,
            })
            .await??
        {
            super::raftlog::RaftLogResponse::QueryResult(mut list) => {
                list.pop().unwrap_or_default()
            }
            _ => return Err(anyhow::anyhow!("RaftLogResponse is error")),
        };
        //2. get membership
        let member_ship = match index_manager
            .send(super::raftindex::RaftIndexRequest::LoadMember)
            .await??
        {
            RaftIndexResponse::MemberShip {
                member,
                member_after_consensus,
                node_addrs,
            } => MemberShip {
                member,
                member_after_consensus,
                node_addrs,
            },
            _ => return Err(anyhow::anyhow!("RaftIndexResponse is error")),
        };
        //3. build writer
        let header = SnapshotHeaderDto {
            last_index,
            last_term: last_log.term,
            member: member_ship.member,
            member_after_consensus: member_ship.member_after_consensus,
            node_addrs: member_ship.node_addrs,
        };
        let (writer, snapshot_id, path) = match snapshot_manager
            .send(RaftSnapshotRequest::NewSnapshot(header.clone()))
            .await??
        {
            RaftSnapshotResponse::NewSnapshot(writer, id, path) => (writer, id, path),
            _ => return Err(anyhow::anyhow!("RaftSnapshotResponse is error")),
        };
        //4. write data
        data_wrap
            .config
            .send(ConfigCmd::BuildSnapshot(writer.clone()))
            .await??;
        data_wrap
            .table
            .send(TableManagerInnerReq::BuildSnapshot(writer.clone()))
            .await??;
        data_wrap
            .namespace
            .send(RaftApplyDataRequest::BuildSnapshot(writer.clone()))
            .await??;

        //5. flush to file
        writer
            .send(super::raftsnapshot::SnapshotWriterRequest::Flush)
            .await??;

        let snapshot_range = SnapshotRange {
            id: snapshot_id,
            end_index: last_index,
        };
        snapshot_manager
            .send(RaftSnapshotRequest::CompleteSnapshot(snapshot_range))
            .await??;
        //log_manager.do_send(RaftLogManagerRequest::SplitOff(last_index));
        Ok((header, path, snapshot_id))
    }
}

impl Actor for StateApplyManager {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        log::info!("StateApplyManager started");
    }
}

impl Inject for StateApplyManager {
    type Context = Context<Self>;

    fn inject(
        &mut self,
        factory_data: bean_factory::FactoryData,
        _factory: bean_factory::BeanFactory,
        ctx: &mut Self::Context,
    ) {
        self.index_manager = factory_data.get_actor();
        self.snapshot_manager = factory_data.get_actor();
        self.log_manager = factory_data.get_actor();
        self.data_wrap = factory_data.get_bean();

        self.init(ctx);
    }
}

#[derive(Message, Debug)]
#[rtype(result = "anyhow::Result<StateApplyResponse>")]
pub enum StateApplyRequest {
    GetLastAppliedLog,
    //ApplyEntries(Vec<Entry<ClientRequest>>),
    ApplyBatchRequest(Vec<ApplyRequestDto>),
    ApplySnapshot { snapshot: Box<tokio::fs::File> },
}

#[derive(Message, Debug)]
#[rtype(result = "anyhow::Result<StateApplyResponse>")]
pub enum StateApplyAsyncRequest {
    BuildSnapshot,
    ApplyRequest(ApplyRequestDto),
}

pub enum StateApplyResponse {
    None,
    Snapshot(SnapshotHeaderDto, Arc<String>, u64),
    LastAppliedLog(u64),
    RaftResponse(ClientResponse),
}

/// raft 数据实施请求
/// 每个类型表数据管理actor都要监听些消息，以支持数据持久化或加载数据
#[derive(Message, Debug)]
#[rtype(result = "anyhow::Result<RaftApplyDataResponse>")]
pub enum RaftApplyDataRequest {
    /// 把镜像数据持久化到指定writer
    BuildSnapshot(Addr<SnapshotWriterActor>),
    /// 把镜像数据加载到对应业务数据管理actor
    LoadSnapshotRecord(SnapshotRecordDto),
    /// 数据加载完成
    LoadCompleted,
}

pub enum RaftApplyDataResponse {
    None,
}

impl Handler<StateApplyRequest> for StateApplyManager {
    type Result = anyhow::Result<StateApplyResponse>;

    fn handle(&mut self, msg: StateApplyRequest, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            /*
            StateApplyRequest::ApplyRequest(request) => {
                self.apply_request_to_state_machine(request)?;
                Ok(StateApplyResponse::None)
            }
             */
            StateApplyRequest::ApplyBatchRequest(requests) => {
                if let Some(req) = requests.last() {
                    self.last_applied_log = req.index;
                }
                for request in requests.into_iter() {
                    self.apply_request_to_state_machine(request)?;
                }
                if let Some(index_manager) = &self.index_manager {
                    index_manager.do_send(super::raftindex::RaftIndexRequest::SaveLastAppliedLog(
                        self.last_applied_log,
                    ));
                }
                Ok(StateApplyResponse::None)
            }
            StateApplyRequest::ApplySnapshot { snapshot } => {
                self.apply_snapshot(ctx, snapshot);
                Ok(StateApplyResponse::None)
            }
            StateApplyRequest::GetLastAppliedLog => {
                Ok(StateApplyResponse::LastAppliedLog(self.last_applied_log))
            }
        }
    }
}

impl Handler<StateApplyAsyncRequest> for StateApplyManager {
    type Result = ResponseActFuture<Self, anyhow::Result<StateApplyResponse>>;

    fn handle(&mut self, msg: StateApplyAsyncRequest, _ctx: &mut Self::Context) -> Self::Result {
        let log_manager = self.log_manager.clone().unwrap();
        let index_manager = self.index_manager.clone().unwrap();
        let snapshot_manager = self.snapshot_manager.clone().unwrap();
        let data_wrap = self.data_wrap.clone().unwrap();
        match &msg {
            StateApplyAsyncRequest::BuildSnapshot => {}
            StateApplyAsyncRequest::ApplyRequest(req) => {
                self.last_applied_log = req.index;
            }
        };
        let last_index = self.last_applied_log;
        let fut = async move {
            match msg {
                StateApplyAsyncRequest::BuildSnapshot => {
                    let (header, path, snapshot_id) = Self::do_build_snapshot(
                        log_manager,
                        index_manager,
                        snapshot_manager,
                        data_wrap,
                        last_index,
                    )
                    .await?;
                    Ok(StateApplyResponse::Snapshot(header, path, snapshot_id))
                }
                StateApplyAsyncRequest::ApplyRequest(req) => {
                    let resp =
                        Self::async_apply_request_to_state_machine(req, &data_wrap, index_manager)
                            .await?;
                    Ok(StateApplyResponse::RaftResponse(resp))
                }
            }
        }
        .into_actor(self)
        .map(|r, _act, _ctx| r);
        Box::pin(fut)
    }
}
