// raft缓存数据

use std::{convert::TryInto, sync::Arc};

use actix::prelude::*;
use bean_factory::{bean, Inject};
use inner_mem_cache::MemCache;

use crate::{now_millis, now_millis_i64};

use self::model::{CacheItemDo, CacheKey, CacheValue};

use super::db::{
    route::TableRoute,
    table::{TableManager, TableManagerQueryReq, TableManagerResult},
};

pub mod model;

lazy_static::lazy_static! {
    static ref CACHE_TABLE_NAME: Arc<String> =  Arc::new("cache".to_string());
}

#[bean(inject)]
pub struct CacheManager {
    cache: MemCache<CacheKey, CacheValue>,
    //default_timeout: i32,
    raft_table_route: Option<Arc<TableRoute>>,
    table_manager: Option<Addr<TableManager>>,
}

impl CacheManager {
    pub fn new() -> Self {
        Self {
            cache: MemCache::default(),
            //default_timeout: 1200,
            raft_table_route: None,
            table_manager: None,
        }
    }

    fn load(&mut self, ctx: &mut Context<Self>) -> anyhow::Result<()> {
        let table_manager = self.table_manager.clone();
        async move {
            if let Some(table_manager) = &table_manager {
                let query_req = TableManagerQueryReq::QueryPageList {
                    table_name: CACHE_TABLE_NAME.clone(),
                    offset: None,
                    limit: None,
                    is_rev: false,
                };
                match table_manager.send(query_req).await?? {
                    TableManagerResult::PageListResult(_, list) => Ok(Some(list)),
                    _ => Ok(None),
                }
            } else {
                Ok(None)
            }
        }
        .into_actor(self)
        .map(
            |result: anyhow::Result<Option<Vec<(Vec<u8>, Vec<u8>)>>>, act, _ctx| match act
                .do_load(result)
            {
                Ok(_) => {}
                Err(e) => log::error!("load cache info error,{}", e.to_string()),
            },
        )
        .wait(ctx);
        Ok(())
    }

    fn do_load(
        &mut self,
        result: anyhow::Result<Option<Vec<(Vec<u8>, Vec<u8>)>>>,
    ) -> anyhow::Result<()> {
        let now = now_millis_i64() as i32;
        if let Ok(Some(list)) = result {
            for (k, v) in list {
                let cache_item = CacheItemDo::from_bytes(&v)?;
                let ttl = cache_item.timeout - now;
                if ttl <= 0 {
                    continue;
                }
                let value: CacheValue = cache_item.try_into()?;
                let key = CacheKey::from_bytes(k, value.get_cache_type() as u8)?;
                self.cache.set(key, value, ttl);
            }
        }
        Ok(())
    }
}

impl Actor for CacheManager {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        log::info!("CacheManager actor started");
    }
}

impl Inject for CacheManager {
    type Context = Context<Self>;

    fn inject(
        &mut self,
        factory_data: bean_factory::FactoryData,
        _factory: bean_factory::BeanFactory,
        ctx: &mut Self::Context,
    ) {
        self.raft_table_route = factory_data.get_bean();
        self.table_manager = factory_data.get_actor();
        //init
        self.load(ctx);
    }
}

#[derive(Message)]
#[rtype(result = "anyhow::Result<CacheManagerResult>")]
pub enum CacheManagerReq {
    Set {
        key: CacheKey,
        value: CacheValue,
        ttl: i32,
    },
    Get(CacheKey),
    UpdateTimeOut {
        key: CacheKey,
        ttl: i32,
    },
}

pub enum CacheManagerResult {
    None,
}

pub enum CacheManagerInnerCtx {}

impl Handler<CacheManagerReq> for CacheManager {
    type Result = ResponseActFuture<Self, anyhow::Result<CacheManagerResult>>;

    fn handle(&mut self, msg: CacheManagerReq, ctx: &mut Self::Context) -> Self::Result {
        todo!()
    }
}
