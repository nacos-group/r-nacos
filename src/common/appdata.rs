use crate::common::AppSysConfig;
use crate::config::core::ConfigActor;
use crate::grpc::bistream_manage::BiStreamManage;
use crate::naming::cluster::node_manage::{InnerNodeManage, NodeManage};
use crate::naming::cluster::route::NamingRoute;
use crate::naming::core::NamingActor;
use crate::raft::cluster::route::ConfigRoute;
use crate::raft::db::route::TableRoute;
use crate::raft::db::table::TableManager;
use crate::raft::network::factory::RaftClusterRequestSender;
use crate::raft::store::core::RaftStore;
use crate::raft::NacosRaft;
use actix::Addr;
use bean_factory::FactoryData;
use std::sync::Arc;

pub struct AppShareData {
    pub config_addr: Addr<ConfigActor>,
    pub naming_addr: Addr<NamingActor>,
    pub bi_stream_manage: Addr<BiStreamManage>,
    pub raft: Arc<NacosRaft>,
    pub raft_store: Arc<RaftStore>,
    pub sys_config: Arc<AppSysConfig>,
    pub config_route: Arc<ConfigRoute>,
    pub cluster_sender: Arc<RaftClusterRequestSender>,
    pub naming_route: Arc<NamingRoute>,
    pub naming_inner_node_manage: Addr<InnerNodeManage>,
    pub naming_node_manage: Arc<NodeManage>,
    pub raft_table_manage: Addr<TableManager>,
    pub raft_table_route: Arc<TableRoute>,
    pub factory_data: FactoryData,
}
