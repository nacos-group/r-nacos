use std::{collections::HashSet, sync::Arc, time::Duration};

use crate::common::actor_utils::{create_actor_at_thread, create_actor_at_thread2};
use crate::grpc::handler::RAFT_ROUTE_REQUEST;
use crate::metrics::core::MetricsManager;
use crate::namespace::NamespaceActor;
use crate::raft::cluster::route::RaftRequestRoute;
use crate::raft::filestore::core::FileStore;
use crate::raft::filestore::raftapply::StateApplyManager;
use crate::raft::filestore::raftdata::RaftDataWrap;
use crate::raft::filestore::raftindex::RaftIndexManager;
use crate::raft::filestore::raftlog::RaftLogManager;
use crate::raft::filestore::raftsnapshot::RaftSnapshotManager;
use crate::{
    common::{appdata::AppShareData, AppSysConfig},
    config::core::ConfigActor,
    grpc::{bistream_manage::BiStreamManage, PayloadUtils},
    naming::{
        cluster::{
            instance_delay_notify::ClusterInstanceDelayNotifyActor,
            node_manage::{InnerNodeManage, NodeManage},
            route::NamingRoute,
        },
        core::NamingActor,
        naming_delay_nofity::DelayNotifyActor,
    },
    raft::{
        cache::{route::CacheRoute, CacheManager},
        cluster::{
            model::RouterRequest,
            route::{ConfigRoute, RaftAddrRouter},
        },
        db::{route::TableRoute, table::TableManager},
        NacosRaft,
        {
            network::{
                core::RaftRouter,
                factory::{RaftClusterRequestSender, RaftConnectionFactory},
            },
            store::ClientRequest,
        },
    },
    user::UserManager,
};
use actix::prelude::*;
use async_raft_ext::{raft::ClientWriteRequest, Config, Raft, RaftStorage};
use bean_factory::{BeanDefinition, BeanFactory, FactoryData};
use chrono::{FixedOffset, Local, Offset};

pub async fn config_factory(sys_config: Arc<AppSysConfig>) -> anyhow::Result<FactoryData> {
    /*
    let db = Arc::new(
        sled::Config::new()
            .path(&sys_config.config_db_dir)
            .mode(sled::Mode::LowSpace)
            .cache_capacity(10 * 1024 * 1024)
            //.flush_every_ms(Some(1000))
            .open()
            .unwrap(),
    );
    factory.register(BeanDefinition::from_obj(db.clone()));
     */
    std::fs::create_dir_all(sys_config.config_db_dir.as_str())?;
    let base_path = Arc::new(sys_config.config_db_dir.clone());
    let factory = BeanFactory::new();
    factory.register(BeanDefinition::from_obj(sys_config.clone()));

    let index_manager = RaftIndexManager::new(base_path.clone());
    let (index_manager, config_addr) = create_actor_at_thread2(index_manager, ConfigActor::new());
    factory.register(BeanDefinition::actor_with_inject_from_obj::<ConfigActor>(
        config_addr.clone(),
    ));
    let naming_addr = NamingActor::create_at_new_system();
    factory.register(BeanDefinition::actor_with_inject_from_obj(
        naming_addr.clone(),
    ));
    factory.register(BeanDefinition::actor_with_inject_from_obj(
        DelayNotifyActor::new().start(),
    ));

    //raft
    let conn_factory = RaftConnectionFactory::new(60).start();
    factory.register(BeanDefinition::actor_from_obj(conn_factory.clone()));
    let cluster_sender = Arc::new(RaftClusterRequestSender::new(
        conn_factory,
        sys_config.clone(),
    ));
    factory.register(BeanDefinition::from_obj(cluster_sender.clone()));

    let log_manager = RaftLogManager::new(base_path.clone(), Some(index_manager.clone()));
    let log_manager = create_actor_at_thread(log_manager);
    let snapshot_manager = RaftSnapshotManager::new(base_path.clone(), Some(index_manager.clone()));
    let apply_manager = StateApplyManager::new();
    let (snapshot_manager, apply_manager) =
        create_actor_at_thread2(snapshot_manager, apply_manager);
    factory.register(BeanDefinition::actor_with_inject_from_obj(
        log_manager.clone(),
    ));
    factory.register(BeanDefinition::actor_with_inject_from_obj(
        index_manager.clone(),
    ));
    factory.register(BeanDefinition::actor_with_inject_from_obj(
        snapshot_manager.clone(),
    ));
    factory.register(BeanDefinition::actor_with_inject_from_obj(
        apply_manager.clone(),
    ));

    let store = Arc::new(FileStore::new(
        sys_config.raft_node_id.to_owned(),
        index_manager,
        snapshot_manager,
        log_manager,
        apply_manager,
    ));
    factory.register(BeanDefinition::from_obj(store.clone()));
    let raft = build_raft(&sys_config, store.clone(), cluster_sender.clone()).await?;
    factory.register(BeanDefinition::from_obj(raft.clone()));
    let table_manage = TableManager::new().start();
    factory.register(BeanDefinition::actor_with_inject_from_obj(
        table_manage.clone(),
    ));

    let raft_addr_router = Arc::new(RaftAddrRouter::new(
        raft.clone(),
        store.clone(),
        sys_config.raft_node_id.to_owned(),
    ));
    factory.register(BeanDefinition::from_obj(raft_addr_router.clone()));
    let table_route = Arc::new(TableRoute::new(
        table_manage.clone(),
        raft_addr_router.clone(),
        cluster_sender.clone(),
    ));
    factory.register(BeanDefinition::from_obj(table_route));
    let config_route = Arc::new(ConfigRoute::new(
        config_addr.clone(),
        raft_addr_router.clone(),
        cluster_sender.clone(),
    ));
    factory.register(BeanDefinition::from_obj(config_route.clone()));

    let naming_inner_node_manage_addr =
        InnerNodeManage::new(sys_config.raft_node_id.to_owned()).start();
    factory.register(BeanDefinition::actor_with_inject_from_obj(
        naming_inner_node_manage_addr.clone(),
    ));
    let naming_node_manage = Arc::new(NodeManage::new(naming_inner_node_manage_addr.clone()));
    factory.register(BeanDefinition::from_obj(naming_node_manage.clone()));
    let naming_route = Arc::new(NamingRoute::new(
        naming_addr.clone(),
        naming_node_manage.clone(),
        cluster_sender.clone(),
    ));
    factory.register(BeanDefinition::from_obj(naming_route.clone()));
    let naming_cluster_delay_notify_addr = ClusterInstanceDelayNotifyActor::new().start();
    factory.register(BeanDefinition::actor_with_inject_from_obj(
        naming_cluster_delay_notify_addr.clone(),
    ));

    let bistream_manage_addr = BiStreamManage::new().start();
    factory.register(BeanDefinition::actor_with_inject_from_obj(
        bistream_manage_addr.clone(),
    ));

    let user_manager = UserManager::new().start();
    factory.register(BeanDefinition::actor_with_inject_from_obj(user_manager));
    let cache_manager = CacheManager::new().start();
    factory.register(BeanDefinition::actor_with_inject_from_obj(
        cache_manager.clone(),
    ));
    let cache_route = Arc::new(CacheRoute::new(
        cache_manager.clone(),
        raft_addr_router.clone(),
        cluster_sender.clone(),
    ));
    factory.register(BeanDefinition::from_obj(cache_route));
    let raft_data_wrap = Arc::new(RaftDataWrap {
        config: config_addr.clone(),
        table: table_manage.clone(),
        //cache: cache_manager.clone(),
    });
    factory.register(BeanDefinition::from_obj(raft_data_wrap));
    let metrics_manager = MetricsManager::new().start();
    factory.register(BeanDefinition::actor_with_inject_from_obj(metrics_manager));
    let namespace_addr = NamespaceActor::new().start();
    factory.register(BeanDefinition::actor_with_inject_from_obj(
        namespace_addr.clone(),
    ));
    let raft_request_route = Arc::new(RaftRequestRoute::new(
        raft_addr_router.clone(),
        cluster_sender.clone(),
        namespace_addr.clone(),
    ));
    factory.register(BeanDefinition::from_obj(raft_request_route));

    Ok(factory.init().await)
}

pub fn build_share_data(factory_data: FactoryData) -> anyhow::Result<Arc<AppShareData>> {
    let sys_config: Arc<AppSysConfig> = factory_data.get_bean().unwrap();
    let timezone_offset =
        if let Some(offset_value) = sys_config.gmt_fixed_offset_hours.map(|e| e * 3600) {
            FixedOffset::east_opt(offset_value).unwrap_or(Local::now().offset().fix())
        } else {
            Local::now().offset().fix()
        };
    let app_data = Arc::new(AppShareData {
        config_addr: factory_data.get_actor().unwrap(),
        naming_addr: factory_data.get_actor().unwrap(),
        bi_stream_manage: factory_data.get_actor().unwrap(),
        raft: factory_data.get_bean().unwrap(),
        raft_store: factory_data.get_bean().unwrap(),
        sys_config,
        config_route: factory_data.get_bean().unwrap(),
        cluster_sender: factory_data.get_bean().unwrap(),
        naming_route: factory_data.get_bean().unwrap(),
        naming_inner_node_manage: factory_data.get_actor().unwrap(),
        naming_node_manage: factory_data.get_bean().unwrap(),
        raft_table_manage: factory_data.get_actor().unwrap(),
        raft_table_route: factory_data.get_bean().unwrap(),
        raft_cache_route: factory_data.get_bean().unwrap(),
        user_manager: factory_data.get_actor().unwrap(),
        cache_manager: factory_data.get_actor().unwrap(),
        metrics_manager: factory_data.get_actor().unwrap(),
        timezone_offset: Arc::new(timezone_offset),
        namespace_addr: factory_data.get_actor().unwrap(),
        raft_request_route: factory_data.get_bean().unwrap(),
        factory_data,
    });
    Ok(app_data)
}

async fn build_raft(
    sys_config: &Arc<AppSysConfig>,
    store: Arc<FileStore>,
    cluster_sender: Arc<RaftClusterRequestSender>,
) -> anyhow::Result<Arc<NacosRaft>> {
    match store.get_last_log_index().await {
        Ok(last_log) => log::info!(
            "[PEEK_RAFT_LOG] raft last log,index:{} term:{}",
            last_log.index,
            last_log.term
        ),
        Err(e) => log::warn!("[PEEK_RAFT_LOG] raft last log is empty,error:{}", e),
    };
    let config = Config::build("rnacos raft".to_owned())
        .heartbeat_interval(1000)
        .election_timeout_min(2500)
        .election_timeout_max(5000)
        .snapshot_policy(async_raft_ext::SnapshotPolicy::LogsSinceLast(
            sys_config.raft_snapshot_log_size,
        ))
        .snapshot_max_chunk_size(3 * 1024 * 1024)
        .validate()
        .unwrap();
    let config = Arc::new(config);
    let network = Arc::new(RaftRouter::new(store.clone(), cluster_sender.clone()));
    let raft = Arc::new(Raft::new(
        sys_config.raft_node_id.to_owned(),
        config,
        network,
        store.clone(),
    ));
    if sys_config.raft_auto_init {
        tokio::spawn(auto_init_raft(store, raft.clone(), sys_config.clone()));
    } else if !sys_config.raft_join_addr.is_empty() {
        tokio::spawn(auto_join_raft(store, sys_config.clone(), cluster_sender));
    }
    Ok(raft)
}

async fn auto_init_raft(
    store: Arc<FileStore>,
    raft: Arc<NacosRaft>,
    sys_config: Arc<AppSysConfig>,
) -> anyhow::Result<()> {
    let state = store.get_initial_state().await?;
    if state.last_log_term == 0 {
        log::info!(
            "auto init raft. node_id:{},addr:{}",
            &sys_config.raft_node_id,
            &sys_config.raft_node_addr
        );
        let mut members = HashSet::new();
        members.insert(sys_config.raft_node_id.to_owned());
        raft.initialize(members).await.ok();
        raft.client_write(ClientWriteRequest::new(ClientRequest::NodeAddr {
            id: sys_config.raft_node_id,
            addr: Arc::new(sys_config.raft_node_addr.to_owned()),
        }))
        .await
        .ok();
        raft.client_write(ClientWriteRequest::new(ClientRequest::Members(vec![
            sys_config.raft_node_id,
        ])))
        .await
        .ok();
    } else if state.membership.all_nodes().len() < 2 {
        // 单节点支持更新集群ip地址
        tokio::time::sleep(Duration::from_millis(500)).await;
        if let Some(node_id) = raft.current_leader().await {
            if node_id == sys_config.raft_node_id {
                raft.client_write(ClientWriteRequest::new(ClientRequest::NodeAddr {
                    id: sys_config.raft_node_id,
                    addr: Arc::new(sys_config.raft_node_addr.to_owned()),
                }))
                .await
                .ok();
            }
        }
    }
    Ok(())
}

async fn auto_join_raft(
    store: Arc<FileStore>,
    sys_config: Arc<AppSysConfig>,
    cluster_sender: Arc<RaftClusterRequestSender>,
) -> anyhow::Result<()> {
    let state = store.get_initial_state().await?;
    if state.last_log_term == 0 {
        //wait for self raft network started
        tokio::time::sleep(Duration::from_millis(500)).await;
        let req = RouterRequest::JoinNode {
            node_id: sys_config.raft_node_id.to_owned(),
            node_addr: Arc::new(sys_config.raft_node_addr.to_owned()),
        };
        let request = serde_json::to_string(&req).unwrap_or_default();
        let payload = PayloadUtils::build_payload(RAFT_ROUTE_REQUEST, request);
        cluster_sender
            .send_request(Arc::new(sys_config.raft_join_addr.to_owned()), payload)
            .await?;
        log::info!(
            "auto join raft,join_addr:{}.node_id:{},addr:{}",
            &sys_config.raft_join_addr,
            &sys_config.raft_node_id,
            &sys_config.raft_node_addr
        );
    }
    Ok(())
}
