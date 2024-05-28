// raft缓存数据

use std::time::Duration;
use std::{convert::TryInto, sync::Arc};

use actix::prelude::*;
use bean_factory::{bean, Inject};
use inner_mem_cache::{MemCache, MemCacheMode};
use ratelimiter_rs::RateLimiter;
use serde::{Deserialize, Serialize};

use crate::common::constant::CACHE_TREE_NAME;
use crate::{common::limiter_utils::LimiterData, now_millis_i64, now_second_i32};

use self::model::{CacheItemDo, CacheKey, CacheValue};

use super::db::{
    route::TableRoute,
    table::{TableManager, TableManagerQueryReq, TableManagerReq, TableManagerResult},
};

pub mod api;
pub mod model;
pub mod route;

#[bean(inject)]
pub struct CacheManager {
    cache: MemCache<CacheKey, CacheValue>,
    //default_timeout: i32,
    raft_table_route: Option<Arc<TableRoute>>,
    table_manager: Option<Addr<TableManager>>,
}

impl Default for CacheManager {
    fn default() -> Self {
        Self::new()
    }
}

type KvPair = (Vec<u8>, Vec<u8>);

impl CacheManager {
    pub fn new() -> Self {
        Self {
            cache: MemCache::new(),
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
                    table_name: CACHE_TREE_NAME.clone(),
                    like_key: None,
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
            |result: anyhow::Result<Option<Vec<KvPair>>>, act, _ctx| match act.do_load(result) {
                Ok(_) => {}
                Err(e) => log::error!("load cache info error,{}", e.to_string()),
            },
        )
        .wait(ctx);
        Ok(())
    }

    fn do_load(&mut self, result: anyhow::Result<Option<Vec<KvPair>>>) -> anyhow::Result<()> {
        let now = now_second_i32();
        if let Ok(Some(list)) = result {
            for (k, v) in list {
                let cache_item = CacheItemDo::from_bytes(&v)?;
                let ttl = cache_item.timeout - now;
                if ttl <= 0 {
                    Self::remove_key(&self.table_manager, k);
                    continue;
                }
                let value: CacheValue = cache_item.try_into()?;
                let key = CacheKey::from_db_key(k)?;
                self.cache.set(key, value, ttl);
            }
        }
        Ok(())
    }

    fn remove_key(table_manager: &Option<Addr<TableManager>>, key: Vec<u8>) {
        if let Some(table_manager) = table_manager.as_ref() {
            let req = TableManagerReq::Remove {
                table_name: CACHE_TREE_NAME.clone(),
                key,
            };
            table_manager.do_send(req);
        }
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
        let table_manager = self.table_manager.clone();
        self.cache.time_out_fn = Some(Arc::new(move |key, _value| {
            CacheManager::remove_key(&table_manager, key.to_key_string().as_bytes().to_vec());
        }));
        //init
        self.load(ctx).ok();
        //增加每10秒触发缓存清理
        self.cache.mode = MemCacheMode::None;
        ctx.run_interval(Duration::from_millis(10000), |act, _| {
            act.cache.clear_time_out();
        });
    }
}

#[derive(Message, Clone, Debug)]
#[rtype(result = "anyhow::Result<CacheManagerResult>")]
pub enum CacheManagerReq {
    Set {
        key: CacheKey,
        value: CacheValue,
        ttl: i32,
    },
    Get(CacheKey),
    Remove(CacheKey),
    NotifyChange {
        key: Vec<u8>,
        value: Vec<u8>,
    },
    NotifyRemove {
        key: Vec<u8>,
    },
}

///只能在主节点执行，才能保证限流的准确性
#[derive(Message, Clone, Debug, Serialize, Deserialize)]
#[rtype(result = "anyhow::Result<CacheManagerResult>")]
pub enum CacheLimiterReq {
    Second {
        key: Arc<String>,
        limit: i32,
    },
    Minutes {
        key: Arc<String>,
        limit: i32,
    },
    Hour {
        key: Arc<String>,
        limit: i32,
    },
    Day {
        key: Arc<String>,
        limit: i32,
    },
    OtherMills {
        key: Arc<String>,
        limit: i32,
        rate_to_ms_conversion: i32,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CacheManagerResult {
    None,
    Value(CacheValue),
    Limiter(bool),
}

pub enum CacheManagerInnerCtx {
    Get(CacheKey),
    Remove(CacheKey),
    Set {
        key: CacheKey,
        value: CacheValue,
        ttl: i32,
    },
    NotifyChange {
        key: Vec<u8>,
        value: Vec<u8>,
    },
    NotifyRemove {
        key: Vec<u8>,
    },
}

impl Handler<CacheManagerReq> for CacheManager {
    type Result = ResponseActFuture<Self, anyhow::Result<CacheManagerResult>>;

    fn handle(&mut self, msg: CacheManagerReq, _ctx: &mut Self::Context) -> Self::Result {
        let raft_table_route = self.raft_table_route.clone();
        let fut = async move {
            match msg {
                CacheManagerReq::Set { key, value, ttl } => {
                    let now = now_second_i32();
                    if let Some(raft_table_route) = &raft_table_route {
                        let mut cache_do: CacheItemDo = value.clone().into();
                        cache_do.timeout = now + ttl;
                        let req = TableManagerReq::Set {
                            table_name: CACHE_TREE_NAME.clone(),
                            key: key.to_key_string().into_bytes(),
                            value: cache_do.to_bytes(),
                            last_seq_id: None,
                        };
                        raft_table_route.request(req).await?;
                    } else {
                        return Err(anyhow::anyhow!("raft_table_route is none "));
                    };
                    Ok(CacheManagerInnerCtx::Set { key, value, ttl })
                }
                CacheManagerReq::Remove(key) => {
                    if let Some(raft_table_route) = &raft_table_route {
                        let req = TableManagerReq::Remove {
                            table_name: CACHE_TREE_NAME.clone(),
                            key: key.to_key_string().into_bytes(),
                        };
                        raft_table_route.request(req).await?;
                    } else {
                        return Err(anyhow::anyhow!("raft_table_route is none "));
                    };
                    Ok(CacheManagerInnerCtx::Remove(key))
                }
                CacheManagerReq::Get(key) => Ok(CacheManagerInnerCtx::Get(key)),
                CacheManagerReq::NotifyChange { key, value } => {
                    Ok(CacheManagerInnerCtx::NotifyChange { key, value })
                }
                CacheManagerReq::NotifyRemove { key } => {
                    Ok(CacheManagerInnerCtx::NotifyRemove { key })
                }
            }
        }
        .into_actor(self)
        .map(
            |inner_ctx: anyhow::Result<CacheManagerInnerCtx>, act, _| match inner_ctx? {
                CacheManagerInnerCtx::Get(key) => match act.cache.get(&key) {
                    Ok(v) => Ok(CacheManagerResult::Value(v)),
                    Err(_) => Ok(CacheManagerResult::None),
                },
                CacheManagerInnerCtx::Remove(key) => {
                    act.cache.remove(&key);
                    Ok(CacheManagerResult::None)
                }
                CacheManagerInnerCtx::Set { key, value, ttl } => {
                    act.cache.set(key, value, ttl);
                    Ok(CacheManagerResult::None)
                }
                CacheManagerInnerCtx::NotifyChange { key, value } => {
                    match act.do_load(Ok(Some(vec![(key, value)]))) {
                        Ok(_) => {}
                        Err(err) => log::error!("do_load error :{}", err.to_string()),
                    };
                    Ok(CacheManagerResult::None)
                }
                CacheManagerInnerCtx::NotifyRemove { key } => {
                    let key = CacheKey::from_db_key(key)?;
                    act.cache.remove(&key);
                    Ok(CacheManagerResult::None)
                }
            },
        );
        Box::pin(fut)
    }
}

impl Handler<CacheLimiterReq> for CacheManager {
    type Result = anyhow::Result<CacheManagerResult>;

    fn handle(&mut self, msg: CacheLimiterReq, _ctx: &mut Self::Context) -> Self::Result {
        let (rate_to_ms_conversion, key, limit) = match msg {
            CacheLimiterReq::Second { key, limit } => (1000, key, limit),
            CacheLimiterReq::Minutes { key, limit } => (60_1000, key, limit),
            CacheLimiterReq::Hour { key, limit } => (60 * 60 * 1000, key, limit),
            CacheLimiterReq::Day { key, limit } => (24 * 60 * 60 * 1000, key, limit),
            CacheLimiterReq::OtherMills {
                key,
                limit,
                rate_to_ms_conversion,
            } => (rate_to_ms_conversion, key, limit),
        };
        let key = CacheKey::new(model::CacheType::String, key);
        let now = now_millis_i64();
        let mut limiter = if let Ok(CacheValue::String(v)) = self.cache.get(&key) {
            let limiter_data: LimiterData = v.as_str().try_into()?;
            limiter_data.to_rate_limiter()
        } else {
            RateLimiter::load(rate_to_ms_conversion, 0, now)
        };
        let r = limiter.acquire(limit, limit as i64);
        if r {
            //只有准入才需要计数，才需要保存

            let limiter_data: LimiterData = limiter.into();
            let cache_value = CacheValue::String(Arc::new(limiter_data.to_string()));
            self.cache.set(
                key.clone(),
                cache_value.clone(),
                rate_to_ms_conversion / 1000,
            );

            let mut cache_do: CacheItemDo = cache_value.into();
            cache_do.timeout = ((now + rate_to_ms_conversion as i64) / 1000) as i32;
            if let Some(table_manager) = self.table_manager.as_ref() {
                let req: TableManagerReq = TableManagerReq::Set {
                    table_name: CACHE_TREE_NAME.clone(),
                    key: key.to_key_string().into_bytes(),
                    value: cache_do.to_bytes(),
                    last_seq_id: None,
                };
                table_manager.do_send(req);
            }
        }
        Ok(CacheManagerResult::Limiter(r))
    }
}
