use std::{sync::Arc, collections::{BTreeMap, HashSet}};

use actix::prelude::*;

use super::NamingRouteAddr;



#[derive(Debug,Clone)]
pub enum NodeStatus {
    Valid,
    Ill,
    Unvalid
}

impl Default for NodeStatus {
    fn default() -> Self {
        Self::Valid
    }
}

#[derive(Default,Debug,Clone)]
pub struct ClusterNode{
    pub id: u64,
    pub index: u64,
    pub addr: Arc<String>,
    pub status: NodeStatus,
}

pub struct InnerNodeManage {
    this_id: u64,
    all_nodes: BTreeMap<u64,ClusterNode>,
}

impl InnerNodeManage {
    pub fn new(this_id:u64) -> Self {
        Self {
            this_id,
            all_nodes: Default::default(),
        }
    }

    fn update_nodes(&mut self,nodes:Vec<(u64,Arc<String>)>) {
        let new_sets:HashSet<u64> = (&nodes).iter().map(|e|e.0.to_owned()).collect();
        let mut dels = vec![];
        for key in self.all_nodes.keys() {
            if !new_sets.contains(key) {
                dels.push(*key);
            }
        }
        for key in dels {
            self.all_nodes.remove(&key);
        }
        for (key,addr) in nodes {
            if let Some(node) = self.all_nodes.get_mut(&key) {
                node.addr =  addr;
            }
            else{
                let node = ClusterNode{
                    id: key,
                    index: 0,
                    addr,
                    status: NodeStatus::Valid,
                };
                self.all_nodes.insert(key, node);
            }
        }
        if !self.all_nodes.contains_key(&self.this_id) {
            let node = self.get_this_node();
            self.all_nodes.insert(self.this_id, node);
        }
        self.update_nodes_index();
    }

    fn update_nodes_index(&mut self) {
        let mut i = 0;
        for value in self.all_nodes.values_mut() {
            value.index = i.to_owned();
            i+=1;
        }
    }

    fn get_this_node(&self) -> ClusterNode {
        if let Some(node) =  self.all_nodes.get(&self.this_id) {
            node.to_owned()
        }
        else{
            ClusterNode {
                id: self.this_id,
                ..Default::default()
            }
        }
    }

    fn get_all_nodes(&self) -> Vec<ClusterNode> {
        if self.all_nodes.len() == 0 {
            vec![self.get_this_node()]
        }
        else{
            self.all_nodes.values().cloned().collect()
        }
    }

}

impl Actor for InnerNodeManage {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        log::info!("InnerNodeManage started!")

        //TODO 定时检测节点的可用性
    }
}

#[derive(Message,Debug)]
#[rtype(result = "anyhow::Result<NodeManageResponse>")]
pub enum NodeManageRequest {
    UpdateNodes(Vec<(u64,Arc<String>)>),
    GetThisNode,
    GetAllNodes,
}

pub enum NodeManageResponse {
    None,
    ThisNode(ClusterNode),
    AllNodes(Vec<ClusterNode>),
}

impl Handler<NodeManageRequest> for InnerNodeManage {
    type Result = anyhow::Result<NodeManageResponse>;

    fn handle(&mut self, msg: NodeManageRequest, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            NodeManageRequest::UpdateNodes(nodes) => {
                self.update_nodes(nodes);
                Ok(NodeManageResponse::None)
            },
            NodeManageRequest::GetThisNode => {
                Ok(NodeManageResponse::ThisNode(self.get_this_node()))
            },
            NodeManageRequest::GetAllNodes => {
                Ok(NodeManageResponse::AllNodes(self.get_all_nodes()))
            },
        }
    }
}


pub struct NodeManage{
    this_id: u64,
    inner_node_manage: Addr<InnerNodeManage>,

}

impl NodeManage {
    pub fn new(this_id:u64,inner_node_manage: Addr<InnerNodeManage>) -> Self {
        Self {
            this_id,
            inner_node_manage,
        }
    }

    pub fn route_addr(&self,hash_value:u64) -> NamingRouteAddr {
        todo!()
    }

    pub fn get_this_node(&self) -> ClusterNode {
        todo!()
    }

    pub fn get_all_nodes(&self) -> Vec<ClusterNode> {
        todo!()
    }

    pub fn get_valid_nodes(&self) -> Vec<ClusterNode> {
        todo!()
    }
    pub fn get_other_valid_nodes(&self) -> Vec<ClusterNode> {
        todo!()
    }
}