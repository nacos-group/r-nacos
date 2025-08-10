use crate::now_millis;
use actix::Message;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LockInstance {
    pub key: String,
    pub expired_time: i64,
    // pub params: Option<HashMap<String, String>>,
    // pub lock_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[rtype(result = "anyhow::Result<bool>")]
pub enum LockRaftReq {
    Acquire { instance: LockInstance },
    Release { instance: LockInstance },
}

pub struct MutexLock {
    pub key: String,
    pub expired_time: Option<i64>,
    pub locked: bool,
}
impl MutexLock {
    pub fn auto_expired(&self) -> bool {
        let now = now_millis() as i64;
        log::info!("expired: {:?}, now: {}", self.expired_time, now);
        self.expired_time.map_or(false, |expired| expired < now)
    }

    pub fn is_clear(&self) -> bool {
        self.auto_expired() || !self.locked
    }
}
