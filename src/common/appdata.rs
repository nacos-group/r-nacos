use crate::common::AppSysConfig;
use crate::config::core::ConfigActor;
use crate::grpc::bistream_manage::BiStreamManage;
use crate::naming::core::NamingActor;
use crate::raft::asyncraft::store::store::AStore;
use crate::raft::NacosRaft;
use actix::Addr;
use std::sync::Arc;
//use crate::raft::store::store::Store;

pub struct AppData {
    pub config_addr: Addr<ConfigActor>,
    pub naming_addr: Addr<NamingActor>,
    pub bi_stream_manage: Addr<BiStreamManage>,
    pub raft: Arc<NacosRaft>,
    pub raft_store: Arc<AStore>,
    pub sys_config: Arc<AppSysConfig>,
}
