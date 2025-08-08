pub mod model;
use crate::lock::model::{LockInstance, LockRaftReq};
use crate::now_millis;
use actix::{Actor, AsyncContext, Context, Handler};
use inner_mem_cache::TimeoutSet;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LockRaftCmd {
    Acquire { instance: LockInstance },
    Release { instance: LockInstance },
}

pub struct LockActor {
    pub(crate) lock_map: HashMap<Arc<String>, LockInstance>,
    pub(crate) expired_lock_set: TimeoutSet<Arc<String>>,
}

impl LockActor {
    pub fn new() -> Self {
        Self {
            lock_map: HashMap::new(),
            expired_lock_set: TimeoutSet::new(),
        }
    }

    /// 定时清理过期锁
    pub fn cleanup_expired(&mut self) {
        let now = now_millis();
        let expired_keys = self.expired_lock_set.timeout(now);
        for key in expired_keys {
            if self.lock_map.remove(&key).is_some() {
                log::info!("Lock [{}] expired and removed", key);
            }
        }
    }

    /// 处理加锁
    fn acquire(&mut self, instance: LockInstance) -> bool {
        let key = Arc::new(instance.key.clone());
        log::info!("Lock [{}] acquired", key.clone());

        if self.lock_map.contains_key(&key) {
            return false;
        }

        // 设置过期
        if let Some(expire_at) = instance.expired_time {
            if expire_at > 0 {
                self.expired_lock_set.add(expire_at as u64, key.clone());
            }
        }

        self.lock_map.insert(key.clone(), instance);
        true
    }

    /// 处理解锁
    fn release(&mut self, instance: LockInstance) -> bool {
        let key = Arc::new(instance.key.clone());
        if self.lock_map.remove(&key).is_some() {
            log::info!("Lock [{}] released", key);
            true
        } else {
            log::info!("Release non-existent lock [{}]", key);
            false
        }
    }
}

impl Actor for LockActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        // 定时清理过期锁
        ctx.run_interval(Duration::from_millis(100), |actor, _ctx| {
            actor.cleanup_expired();
        });
    }
}

impl Handler<LockRaftReq> for LockActor {
    type Result = anyhow::Result<bool>;

    fn handle(&mut self, msg: LockRaftReq, _ctx: &mut Self::Context) -> Self::Result {
        Ok(match msg {
            LockRaftReq::Acquire { instance } => self.acquire(instance),
            LockRaftReq::Release { instance } => self.release(instance),
        })
    }
}

