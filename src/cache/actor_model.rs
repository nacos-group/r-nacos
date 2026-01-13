use crate::cache::model::{CacheKey, CacheValue};
use crate::now_second_i32;
use actix::Message;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CacheSetParam {
    pub key: CacheKey,
    pub value: CacheValue,
    pub ttl: i32,
    pub now: i32,
    pub nx: bool,
    pub xx: bool,
}

impl CacheSetParam {
    pub fn new(key: CacheKey, value: CacheValue) -> Self {
        CacheSetParam {
            key,
            value,
            ttl: -1,
            now: 0,
            nx: false,
            xx: false,
        }
    }

    pub fn new_with_ttl(key: CacheKey, value: CacheValue, ttl: i32) -> Self {
        let now = now_second_i32();
        CacheSetParam {
            key,
            value,
            ttl,
            now,
            nx: false,
            xx: false,
        }
    }
}

/// 本节点查询
#[derive(Message, Clone, Debug)]
#[rtype(result = "anyhow::Result<CacheManagerResult>")]
pub enum CacheManagerLocalReq {
    Get(CacheKey),
    Exists(CacheKey),
    Ttl(CacheKey),
}

/// raft请求
#[derive(Message, Clone, Debug, Serialize, Deserialize)]
#[rtype(result = "anyhow::Result<CacheManagerRaftResult>")]
pub enum CacheManagerRaftReq {
    Set(CacheSetParam),
    GetSet(CacheSetParam),
    Remove(CacheKey),
    Expire(CacheKey, i32),
    Incr(CacheKey, i32),
    Decr(CacheKey, i32),
    Get(CacheKey),
    Exists(CacheKey),
    Ttl(CacheKey),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CacheManagerRaftResult {
    Ok,
    Nil,
    None,
    Value(CacheValue),
    Exists(bool),
    Ttl(i32),
}

pub type CacheManagerResult = CacheManagerRaftResult;
