///目前的接口提供给测试验证使用，后续需要按需调整
use std::sync::Arc;

use actix_web::{
    web::{self, Data, Json},
    Responder,
};
use serde::{Deserialize, Serialize};

use crate::common::appdata::AppShareData;

use super::{
    model::{CacheKey, CacheType, CacheValue},
    CacheManagerReq,
};

#[derive(Debug, Deserialize, Serialize)]
pub struct StringCacheDto {
    pub key: Arc<String>,
    pub value: Option<String>,
    pub ttl: Option<i32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ValueResult {
    pub success: bool,
    pub value: Arc<String>,
}

pub async fn set_cache(
    app: Data<Arc<AppShareData>>,
    web::Form(param): web::Form<StringCacheDto>,
) -> actix_web::Result<impl Responder> {
    let req = CacheManagerReq::Set {
        key: CacheKey::new(CacheType::String, param.key),
        value: CacheValue::String(Arc::new(param.value.unwrap_or_default())),
        ttl: param.ttl.unwrap_or(1200),
    };
    app.cache_manager.send(req).await.ok();
    Ok("{\"ok\":1}")
}

pub async fn remove_cache(
    app: Data<Arc<AppShareData>>,
    web::Form(param): web::Form<StringCacheDto>,
) -> actix_web::Result<impl Responder> {
    let req = CacheManagerReq::Remove(CacheKey::new(CacheType::String, param.key));
    app.cache_manager.send(req).await.ok();
    Ok("{\"ok\":1}")
}

pub async fn get_cache(
    app: Data<Arc<AppShareData>>,
    web::Query(param): web::Query<StringCacheDto>,
) -> actix_web::Result<impl Responder> {
    let req = CacheManagerReq::Get(CacheKey::new(CacheType::String, param.key));
    match app.cache_manager.send(req).await.unwrap().unwrap() {
        super::CacheManagerResult::Value(v) => {
            let vstr = match v {
                CacheValue::String(v) => v,
                CacheValue::Map(m) => Arc::new(serde_json::to_string(&m).unwrap_or_default()),
                CacheValue::UserSession(m) => {
                    Arc::new(serde_json::to_string(&m).unwrap_or_default())
                }
                CacheValue::ApiTokenSession(m) => {
                    Arc::new(serde_json::to_string(&m).unwrap_or_default())
                }
            };
            Ok(Json(ValueResult {
                value: vstr,
                success: true,
            }))
        }
        _ => Ok(Json(ValueResult {
            value: Arc::new("not found key value".to_owned()),
            success: false,
        })),
    }
}
