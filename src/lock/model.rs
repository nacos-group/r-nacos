use actix::Message;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LockInstance {
    pub key: String,
    pub expired_time: Option<i64>,
    // pub params: Option<HashMap<String, String>>,
    // pub lock_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[rtype(result = "anyhow::Result<bool>")]
pub enum LockRaftReq {
    Acquire { instance: LockInstance },
    Release { instance: LockInstance },
}