use std::{collections::HashSet, sync::Arc, time::Duration};

use crate::{
    common::{appdata::AppShareData, AppSysConfig},
    config::core::{ConfigActor},
    grpc::{bistream_manage::BiStreamManage, PayloadUtils},
    naming::{
        cluster::{
            instance_delay_notify::{ClusterInstanceDelayNotifyActor},
            node_manage::{InnerNodeManage, NodeManage},
            route::NamingRoute,
        },
        core::{NamingActor}, naming_delay_nofity::DelayNotifyActor,
    },
    raft::{
        cluster::{
            model::RouterRequest,
            route::{ConfigRoute, RaftAddrRouter},
        },
        NacosRaft,
        {
            network::{
                core::RaftRouter,
                factory::{RaftClusterRequestSender, RaftConnectionFactory},
            },
            store::{core::RaftStore, ClientRequest},
        },
    },
};
use actix::prelude::*;
use async_raft_ext::{raft::ClientWriteRequest, Config, Raft, RaftStorage};
use bean_factory::{BeanFactory, BeanDefinition};

pub fn build_share_data(sys_config: Arc<AppSysConfig>) -> anyhow::Result<Arc<AppShareData>> {
    let db = Arc::new(
        sled::Config::new()
            .path(&sys_config.config_db_dir)
            .mode(sled::Mode::LowSpace)
            .cache_capacity(10 * 1024 * 1024)
            //.flush_every_ms(Some(1000))
            .open()
            .unwrap(),
    );
    let factory = BeanFactory::new();
    factory.register(BeanDefinition::from_obj(sys_config.clone()));
    factory.register(BeanDefinition::from_obj(db.clone()));

    let config_addr = ConfigActor::new(db.clone()).start();
    factory.register(BeanDefinition::actor_with_inject_from_obj::<ConfigActor>(config_addr.clone()));
    let naming_addr = NamingActor::create_at_new_system();
    factory.register(BeanDefinition::actor_with_inject_from_obj(naming_addr.clone()));
    factory.register(BeanDefinition::actor_with_inject_from_obj(DelayNotifyActor::new().start()));

    let store = Arc::new(RaftStore::new(
        sys_config.raft_node_id.to_owned(),
        db,
        config_addr.clone(),
    ));
    factory.register(BeanDefinition::from_obj(store.clone()));
    let conn_factory = RaftConnectionFactory::new(60).start();
    factory.register(BeanDefinition::actor_from_obj(conn_factory.clone()));
    let cluster_sender = Arc::new(RaftClusterRequestSender::new(conn_factory));
    factory.register(BeanDefinition::from_obj(cluster_sender.clone()));
    let raft = build_raft(&sys_config, store.clone(), cluster_sender.clone())?;
    factory.register(BeanDefinition::from_obj(raft.clone()));

    let raft_addr_router = Arc::new(RaftAddrRouter::new(
        raft.clone(),
        store.clone(),
        sys_config.raft_node_id.to_owned(),
    ));
    factory.register(BeanDefinition::from_obj(raft_addr_router.clone()));
    let config_route = Arc::new(ConfigRoute::new(
        config_addr.clone(),
        raft_addr_router,
        cluster_sender.clone(),
    ));
    factory.register(BeanDefinition::from_obj(config_route.clone()));

    let naming_inner_node_manage_addr =
        InnerNodeManage::new(sys_config.raft_node_id.to_owned()).start();
    factory.register(BeanDefinition::actor_with_inject_from_obj(naming_inner_node_manage_addr.clone()));
    store.set_naming_manage_addr(naming_inner_node_manage_addr.clone());
    let naming_node_manage = Arc::new(NodeManage::new(naming_inner_node_manage_addr.clone()));
    factory.register(BeanDefinition::from_obj(naming_node_manage.clone()));
    let naming_route = Arc::new(NamingRoute::new(
        naming_addr.clone(),
        naming_node_manage.clone(),
        cluster_sender.clone(),
    ));
    factory.register(BeanDefinition::from_obj(naming_route.clone()));
    let naming_cluster_delay_notify_addr = ClusterInstanceDelayNotifyActor::new().start();
    factory.register(BeanDefinition::actor_with_inject_from_obj(naming_cluster_delay_notify_addr.clone()));

    let bistream_manage_addr = BiStreamManage::new().start();
    factory.register(BeanDefinition::actor_with_inject_from_obj(bistream_manage_addr.clone()));

    let app_data = Arc::new(AppShareData {
        config_addr,
        naming_addr,
        bi_stream_manage: bistream_manage_addr,
        raft,
        raft_store: store,
        sys_config,
        config_route,
        cluster_sender,
        naming_route,
        naming_inner_node_manage: naming_inner_node_manage_addr,
        naming_node_manage,
    });
    factory.register(BeanDefinition::from_obj(app_data.clone()));
    factory.init();
    Ok(app_data)
}

fn build_raft(
    sys_config: &Arc<AppSysConfig>,
    store: Arc<RaftStore>,
    cluster_sender: Arc<RaftClusterRequestSender>,
) -> anyhow::Result<Arc<NacosRaft>> {
    let config = Config::build("rnacos raft".to_owned())
        .heartbeat_interval(500)
        .election_timeout_min(1500)
        .election_timeout_max(3000)
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
    store: Arc<RaftStore>,
    raft: Arc<NacosRaft>,
    sys_config: Arc<AppSysConfig>,
) -> anyhow::Result<()> {
    let state = store.get_initial_state().await?;
    if state.last_log_term == 0 && state.last_log_index == 0 {
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
    store: Arc<RaftStore>,
    sys_config: Arc<AppSysConfig>,
    cluster_sender: Arc<RaftClusterRequestSender>,
) -> anyhow::Result<()> {
    let state = store.get_initial_state().await?;
    if state.last_log_term == 0 && state.last_log_index == 0 {
        //wait for self raft network started
        tokio::time::sleep(Duration::from_millis(500)).await;
        let req = RouterRequest::JoinNode {
            node_id: sys_config.raft_node_id.to_owned(),
            node_addr: Arc::new(sys_config.raft_node_addr.to_owned()),
        };
        let request = serde_json::to_string(&req).unwrap_or_default();
        let payload = PayloadUtils::build_payload("RaftRouteRequest", request);
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
