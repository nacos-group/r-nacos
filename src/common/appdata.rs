use crate::common::AppSysConfig;
use crate::config::core::ConfigActor;
use crate::grpc::bistream_manage::BiStreamManage;
use crate::health::core::HealthManager;
use crate::ldap::core::LdapManager;
use crate::mcp::core::McpManager;
use crate::mcp::sse_manage::SseStreamManager;
use crate::metrics::core::MetricsManager;
use crate::namespace::NamespaceActor;
use crate::naming::cluster::node_manage::{InnerNodeManage, NodeManage};
use crate::naming::cluster::route::NamingRoute;
use crate::naming::core::NamingActor;
use crate::raft::cache::route::CacheRoute;
use crate::raft::cache::CacheManager;
use crate::raft::cluster::route::{ConfigRoute, RaftRequestRoute};
use crate::raft::db::route::TableRoute;
use crate::raft::db::table::TableManager;
use crate::raft::filestore::core::FileStore;
use crate::raft::network::factory::RaftClusterRequestSender;
use crate::raft::NacosRaft;
use crate::sequence::core::SequenceDbManager;
use crate::sequence::SequenceManager;
use crate::transfer::reader::TransferImportManager;
use crate::transfer::writer::TransferWriterManager;
use crate::user::UserManager;
use actix::Addr;
use bean_factory::FactoryData;
use chrono::FixedOffset;
use std::sync::Arc;

pub struct AppShareData {
    pub config_addr: Addr<ConfigActor>,
    pub naming_addr: Addr<NamingActor>,
    pub bi_stream_manage: Addr<BiStreamManage>,
    pub raft: Arc<NacosRaft>,
    pub raft_store: Arc<FileStore>,
    pub sys_config: Arc<AppSysConfig>,
    pub config_route: Arc<ConfigRoute>,
    pub cluster_sender: Arc<RaftClusterRequestSender>,
    pub naming_route: Arc<NamingRoute>,
    pub naming_inner_node_manage: Addr<InnerNodeManage>,
    pub naming_node_manage: Arc<NodeManage>,
    pub raft_table_manage: Addr<TableManager>,
    pub raft_table_route: Arc<TableRoute>,
    pub raft_cache_route: Arc<CacheRoute>,
    pub factory_data: FactoryData,
    pub user_manager: Addr<UserManager>,
    pub cache_manager: Addr<CacheManager>,
    pub timezone_offset: Arc<FixedOffset>,
    pub metrics_manager: Addr<MetricsManager>,
    pub namespace_addr: Addr<NamespaceActor>,
    pub raft_request_route: Arc<RaftRequestRoute>,
    pub transfer_writer_manager: Addr<TransferWriterManager>,
    pub transfer_import_manager: Addr<TransferImportManager>,
    pub health_manager: Addr<HealthManager>,
    pub ldap_manager: Addr<LdapManager>,
    pub sequence_db_manager: Addr<SequenceDbManager>,
    pub sequence_manager: Addr<SequenceManager>,
    pub mcp_manager: Addr<McpManager>,
    pub sse_stream_manager: Addr<SseStreamManager>,
    pub common_client: reqwest::Client,
}
