use actix::prelude::*;
use crate::config::core::ConfigActor;
use crate::raft::cache::CacheManager;
use crate::raft::db::table::TableManager;

#[derive(Clone)]
pub struct RaftDataWrap {
    pub(crate) config: Addr<ConfigActor>,
    pub(crate) table: Addr<TableManager>,
    pub(crate) cache: Addr<CacheManager>,
}

impl RaftDataWrap{
    pub fn new(config: Addr<ConfigActor>, table: Addr<TableManager>, cache: Addr<CacheManager>) -> Self {
        Self {
            config,
            table,
            cache,
        }
    }
}
