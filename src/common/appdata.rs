use std::sync::Arc;
use actix::Addr;
use crate::common::AppSysConfig;
use crate::config::core::ConfigActor;
use crate::grpc::bistream_manage::BiStreamManage;
use crate::naming::core::NamingActor;
use crate::raft::NacosRaft;

pub struct AppData {
    pub config_addr : Addr<ConfigActor>,
    pub naming_addr : Addr<NamingActor>,
    pub bi_stream_manage: Addr<BiStreamManage>,
    pub raft : NacosRaft,
    pub sys_config: Arc<AppSysConfig>,
}