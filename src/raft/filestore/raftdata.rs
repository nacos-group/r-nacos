use crate::config::core::ConfigActor;
use crate::lock::LockActor;
use crate::namespace::NamespaceActor;
use crate::raft::cache::CacheManager;
use crate::raft::db::table::TableManager;
use actix::prelude::*;

#[derive(Clone)]
pub struct RaftDataWrap {
    pub(crate) config: Addr<ConfigActor>,
    pub(crate) table: Addr<TableManager>,
    pub(crate) namespace: Addr<NamespaceActor>,
    pub(crate) lock: Addr<LockActor>,
    //pub(crate) cache: Addr<CacheManager>,
}

impl RaftDataWrap {
    pub fn new(
        config: Addr<ConfigActor>,
        table: Addr<TableManager>,
        namespace: Addr<NamespaceActor>,
        lock: Addr<LockActor>,
        _cache: Addr<CacheManager>,
    ) -> Self {
        Self {
            config,
            table,
            namespace,
            lock
            //cache,
        }
    }
}
