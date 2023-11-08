// raft缓存数据

use std::sync::Arc;

use inner_mem_cache::MemCache;
use actix::prelude::*;

use self::model::{CacheKey, CacheValue};

use super::db::{route::TableRoute, table::TableManager};

pub mod model;


pub struct CacheManager {
    cache: MemCache<CacheKey,CacheValue>,
    default_timeout: i32,
    raft_table_route: Option<Arc<TableRoute>>,
    table_manager: Option<Addr<TableManager>>,
}

impl CacheManager{
    pub fn new() -> Self {
        Self { 
            cache: MemCache::default(), 
            default_timeout: 1200,
            raft_table_route: None,
            table_manager: None,
        }
    }

    fn update_timeout(&mut self, key: &CacheKey) {
        self.cache.update_time_out(key, self.default_timeout)
    }
}