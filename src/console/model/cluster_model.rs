use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::naming::cluster::node_manage::{ClusterNode, NodeStatus};

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ClusterNodeInfo {
    pub node_id: u64,
    pub addr: Arc<String>,
    pub current_node: bool,
    pub raft_leader: bool,
    pub distro_valid: bool,
}

impl From<ClusterNode> for ClusterNodeInfo {
    fn from(value: ClusterNode) -> Self {
        Self {
            node_id: value.id,
            addr: value.addr,
            raft_leader: false,
            current_node: false,
            distro_valid: value.is_local || value.status == NodeStatus::Valid,
        }
    }
}
