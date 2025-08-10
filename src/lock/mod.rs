pub mod model;
use crate::lock::model::{LockInstance, LockRaftReq, MutexLock};
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
    lock_map: HashMap<String, MutexLock>,
}

impl LockActor {
    pub fn new() -> Self {
        Self {
            lock_map: HashMap::new(),
        }
    }

    /// 处理加锁
    fn acquire(&mut self, instance: LockInstance) -> bool {
        let key = instance.key.clone();
        if let Some(old) = self.lock_map.get(&key) {
            if !old.auto_expired() {
                return false;
            }
        }

        self.lock_map.insert(
            key.clone(),
            MutexLock {
                key: instance.key,
                expired_time: if instance.expired_time == -1 {
                    None
                } else {
                    Some(instance.expired_time + now_millis() as i64)
                },
                locked: true,
            },
        );

        log::info!("Lock [{}] acquired", key);
        true
    }

    /// 处理解锁
    fn release(&mut self, instance: LockInstance) -> bool {
        let key = instance.key.clone();

        match self.lock_map.get_mut(&key) {
            Some(lock) => {
                lock.locked = false;
                log::info!("Lock [{}] released", &key);
                if lock.is_clear() {
                    self.lock_map.remove(&key);
                }
                true
            }
            None => {
                log::info!("Release non-existent lock [{}]", &key);
                false
            }
        }
    }
}

impl Actor for LockActor {
    type Context = Context<Self>;
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
