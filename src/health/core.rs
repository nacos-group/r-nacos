use crate::config::core::ConfigActor;
use crate::health::model::{
    CheckHealthResult, HealthBackRequest, HealthCheckItem, HealthCheckRequest, HealthCheckType,
    HealthManagerRequest, HealthManagerResponse,
};
use crate::naming::core::NamingActor;
use crate::now_millis;
use crate::raft::cache::CacheManager;
use crate::raft::filestore::raftapply::StateApplyManager;
use crate::raft::filestore::raftindex::RaftIndexManager;
use crate::raft::filestore::raftlog::RaftLogManager;
use crate::raft::NacosRaft;
use crate::user::UserManager;
use actix::prelude::*;
use bean_factory::{bean, BeanFactory, FactoryData, Inject};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

#[bean(inject)]
#[derive(Clone, Default)]
pub struct HealthManager {
    config_actor: Option<Addr<ConfigActor>>,
    naming_actor: Option<Addr<NamingActor>>,
    user_actor: Option<Addr<UserManager>>,
    cache_actor: Option<Addr<CacheManager>>,
    raft_index_manager: Option<Addr<RaftIndexManager>>,
    raft_log_manager: Option<Addr<RaftLogManager>>,
    raft_apply_manager: Option<Addr<StateApplyManager>>,
    raft: Option<Arc<NacosRaft>>,
    health_item_map: HashMap<HealthCheckType, HealthCheckItem>,
}

impl HealthManager {
    pub fn new() -> Self {
        let mut health_item_map = HashMap::new();
        let now = now_millis();
        let common_timeout = 5500;
        health_item_map.insert(
            HealthCheckType::Config,
            HealthCheckItem::new(HealthCheckType::Config, common_timeout, now),
        );
        health_item_map.insert(
            HealthCheckType::Naming,
            HealthCheckItem::new(HealthCheckType::Naming, common_timeout, now),
        );
        health_item_map.insert(
            HealthCheckType::User,
            HealthCheckItem::new(HealthCheckType::User, common_timeout, now),
        );
        health_item_map.insert(
            HealthCheckType::Cache,
            HealthCheckItem::new(HealthCheckType::Cache, common_timeout, now),
        );
        health_item_map.insert(
            HealthCheckType::RaftStoreLog,
            HealthCheckItem::new(HealthCheckType::RaftStoreLog, common_timeout, now),
        );
        health_item_map.insert(
            HealthCheckType::RaftStoreIndex,
            HealthCheckItem::new(HealthCheckType::RaftStoreIndex, common_timeout, now),
        );
        health_item_map.insert(
            HealthCheckType::RaftStoreApply,
            HealthCheckItem::new(HealthCheckType::RaftStoreApply, common_timeout, now),
        );
        health_item_map.insert(
            HealthCheckType::RaftCluster,
            HealthCheckItem::new(HealthCheckType::RaftCluster, 12500, now),
        );
        Self {
            config_actor: None,
            naming_actor: None,
            user_actor: None,
            cache_actor: None,
            raft_index_manager: None,
            raft_log_manager: None,
            raft_apply_manager: None,
            raft: None,
            health_item_map,
        }
    }

    /// 测试健康状态成功后更新最新成功的时间
    pub fn update_success_status(&mut self, check_type: HealthCheckType) {
        //for debug
        //log::info!("Health check success: {:?}", check_type);
        if let Some(v) = self.health_item_map.get_mut(&check_type) {
            v.last_success_time = now_millis();
        }
    }

    fn check_raft(&self) -> bool {
        if let Some(raft) = &self.raft {
            let metrics = raft.metrics().borrow().clone();
            metrics.state.is_leader() || metrics.state.is_follower()
        } else {
            false
        }
    }

    fn do_check(&mut self, ctx: &mut Context<Self>) -> anyhow::Result<()> {
        let self_addr = ctx.address();
        if let Some(config) = self.config_actor.as_ref() {
            config.do_send(HealthCheckRequest::Ping(self_addr.clone()));
        }
        if let Some(naming) = self.naming_actor.as_ref() {
            naming.do_send(HealthCheckRequest::Ping(self_addr.clone()));
        }
        if let Some(cache) = self.cache_actor.as_ref() {
            cache.do_send(HealthCheckRequest::Ping(self_addr.clone()));
        }
        if let Some(user) = self.user_actor.as_ref() {
            user.do_send(HealthCheckRequest::Ping(self_addr.clone()));
        }
        // raft集群
        if let Some(raft_index) = self.raft_index_manager.as_ref() {
            raft_index.do_send(HealthCheckRequest::Ping(self_addr.clone()));
        }
        if let Some(raft_log) = self.raft_log_manager.as_ref() {
            raft_log.do_send(HealthCheckRequest::Ping(self_addr.clone()));
        }
        if let Some(raft_apply) = self.raft_apply_manager.as_ref() {
            raft_apply.do_send(HealthCheckRequest::Ping(self_addr.clone()));
        }
        if self.check_raft() {
            self.update_success_status(HealthCheckType::RaftCluster);
        }
        Ok(())
    }

    fn heartbeat(&mut self, ctx: &mut Context<Self>) {
        ctx.run_later(Duration::from_millis(2000), |act, ctx| {
            act.do_check(ctx).ok();
            act.heartbeat(ctx);
        });
    }
}

impl Actor for HealthManager {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        log::info!("HealthManager started");
    }
}

impl Inject for HealthManager {
    type Context = Context<Self>;

    fn inject(
        &mut self,
        factory_data: FactoryData,
        _factory: BeanFactory,
        ctx: &mut Self::Context,
    ) {
        self.config_actor = factory_data.get_actor();
        self.naming_actor = factory_data.get_actor();
        self.user_actor = factory_data.get_actor();
        self.cache_actor = factory_data.get_actor();
        self.raft_apply_manager = factory_data.get_actor();
        self.raft_log_manager = factory_data.get_actor();
        self.raft_index_manager = factory_data.get_actor();
        self.raft = factory_data.get_bean();
        self.heartbeat(ctx);
    }
}

impl Handler<HealthBackRequest> for HealthManager {
    type Result = anyhow::Result<()>;

    fn handle(&mut self, msg: HealthBackRequest, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            HealthBackRequest::Pong(check_type) => {
                self.update_success_status(check_type);
            }
        }
        Ok(())
    }
}

impl Handler<HealthManagerRequest> for HealthManager {
    type Result = anyhow::Result<HealthManagerResponse>;

    fn handle(&mut self, _msg: HealthManagerRequest, _ctx: &mut Self::Context) -> Self::Result {
        let now = now_millis();
        for item in self.health_item_map.values() {
            let result = item.check(now);
            if !result.is_success() {
                return Ok(HealthManagerResponse::StatusResult(result));
            }
        }
        Ok(HealthManagerResponse::StatusResult(
            CheckHealthResult::Success,
        ))
    }
}
