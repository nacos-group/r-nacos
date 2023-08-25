use std::sync::Arc;

use crate::{naming::core::NamingActor, raft::network::factory::RaftClusterRequestSender};

use super::node_manage::NodeManage;

use actix::prelude::*;

#[derive(Clone, Debug)]
pub struct NamingRoute {
    config_addr: Addr<NamingActor>,
    raft_addr_route: Arc<NodeManage>,
    cluster_sender: Arc<RaftClusterRequestSender>,
}