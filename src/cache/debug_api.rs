///目前的接口提供给测试验证使用，后续需要按需调整
use std::sync::Arc;

use super::model::{CacheKey, CacheType, CacheValue};
use crate::cache::actor_model::{
    CacheManagerLocalReq, CacheManagerRaftReq, CacheManagerRaftResult, CacheSetParam,
};
use crate::common::datetime_utils::now_second_i32;
use crate::common::share_data::ShareData;
use crate::raft::store::ClientRequest;
use actix_web::web::ServiceConfig;
use actix_web::{
    web::{self, Data, Json},
    Responder,
};
use serde::{Deserialize, Serialize};

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
    app: Data<Arc<ShareData>>,
    web::Form(param): web::Form<StringCacheDto>,
) -> actix_web::Result<impl Responder> {
    let mut set_info = CacheSetParam::new(
        CacheKey::new(CacheType::String, param.key),
        CacheValue::String(Arc::new(param.value.unwrap_or_default())),
    );
    set_info.ttl = param.ttl.unwrap_or(1200);
    set_info.now = now_second_i32();
    let req = CacheManagerRaftReq::Set(set_info);
    app.raft_request_route
        .request(ClientRequest::CacheReq { req })
        .await
        .ok();
    Ok("{\"ok\":1}")
}

pub async fn remove_cache(
    app: Data<Arc<ShareData>>,
    web::Form(param): web::Form<StringCacheDto>,
) -> actix_web::Result<impl Responder> {
    let req = CacheManagerRaftReq::Remove(CacheKey::new(CacheType::String, param.key));
    app.raft_request_route
        .request(ClientRequest::CacheReq { req })
        .await
        .ok();
    Ok("{\"ok\":1}")
}

pub async fn get_cache(
    app: Data<Arc<ShareData>>,
    web::Query(param): web::Query<StringCacheDto>,
) -> actix_web::Result<impl Responder> {
    let req = CacheManagerLocalReq::Get(CacheKey::new(CacheType::String, param.key));
    match app.cache_manager.send(req).await.unwrap().unwrap() {
        CacheManagerRaftResult::Value(v) => {
            let value = match v {
                CacheValue::Number(v) => Arc::new(v.to_string()),
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
                value,
                success: true,
            }))
        }
        _ => Ok(Json(ValueResult {
            value: Arc::new("not found key value".to_owned()),
            success: false,
        })),
    }
}

pub fn cache_debug_api_config(config: &mut ServiceConfig) {
    config.service(
        web::scope("/rnacos/debug/cache")
            .service(web::resource("/set").route(web::post().to(set_cache)))
            .service(web::resource("/remove").route(web::post().to(remove_cache)))
            .service(web::resource("/get").route(web::get().to(get_cache))),
    );
}
