use crate::cache::model::{CacheKey, CacheValue};
use actix::Message;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SetInfo {
    pub key: CacheKey,
    pub value: CacheValue,
    pub ttl: i32,
    pub now: i32,
    pub nx: bool,
    pub xx: bool,
}

impl SetInfo {
    pub fn new(key: CacheKey, value: CacheValue) -> Self {
        SetInfo {
            key,
            value,
            ttl: -1,
            now: 0,
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
    Set(SetInfo),
    GetSet(SetInfo),
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
