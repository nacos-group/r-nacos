use crate::cache::actor_model::CacheManagerRaftResult;
use crate::common::appdata::AppShareData;
use crate::common::constant::{AUTHORIZATION_HEADER, EMPTY_ARC_STRING};
use crate::common::datetime_utils;
use crate::common::model::TokenSession;
use crate::metrics::core::MetricsManager;
use crate::metrics::metrics_key::MetricsKey;
use crate::metrics::model::{MetricsItem, MetricsRecord, MetricsRequest};
use crate::raft::cache::model::{CacheKey, CacheType};
use crate::raft::cluster::model::{RouterRequest, RouterResponse};
use crate::user::permission::UserRole;
use actix::Addr;
use actix_http::body::EitherBody;
use actix_http::HttpMessage;
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::{dev, web, Error, HttpResponse};
use futures_util::future::LocalBoxFuture;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::future::{ready, Ready};
use std::sync::Arc;
use std::time::SystemTime;

lazy_static::lazy_static! {
    pub static ref IGNORE_PATH: Vec<&'static str> = vec![
        "/nacos/v1/auth/login", "/nacos/v1/auth/users/login","/nacos/v3/auth/user/login" ,"/nacos/metrics"
    ];
    pub static ref API_PATH: Regex = Regex::new(r"(?i)/nacos/.*").unwrap();
    pub static ref IGNORE_METRICS_PATH: Vec<&'static str> = vec![
        "/nacos/v1/cs/configs/listener"
    ];
    //pub static ref PARM_AUTH_TOKEN: Regex = Regex::new(r"accessToken=(\w*)").unwrap();
}

#[derive(Clone)]
pub struct ApiCheckAuth {
    app_share_data: Arc<AppShareData>,
}

impl ApiCheckAuth {
    pub fn new(app_share_data: Arc<AppShareData>) -> Self {
        Self { app_share_data }
    }
}

impl<S, B> Transform<S, ServiceRequest> for ApiCheckAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = ApiCheckAuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(ApiCheckAuthMiddleware {
            service: Arc::new(service),
            app_share_data: self.app_share_data.clone(),
        }))
    }
}

#[derive(Clone)]
pub struct ApiCheckAuthMiddleware<S> {
    service: Arc<S>,
    app_share_data: Arc<AppShareData>,
}

impl<S, B> Service<ServiceRequest> for ApiCheckAuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    dev::forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let start = SystemTime::now();
        let mut request = req;
        let enable_auth = self.app_share_data.sys_config.openapi_enable_auth;
        let path = request.path();
        let is_check_path = if enable_auth {
            API_PATH.is_match(path) && !IGNORE_PATH.contains(&path)
        } else {
            true
        };
        let ignore_metrics = IGNORE_METRICS_PATH.contains(&path);
        let app_share_data = self.app_share_data.clone();
        let service = self.service.clone();
        Box::pin(async move {
            let offset = &app_share_data.timezone_offset;
            let token = if enable_auth && is_check_path {
                if let Some(v) = request.headers().get(AUTHORIZATION_HEADER) {
                    Arc::new(v.to_str().unwrap_or_default().to_owned())
                } else if let Ok(info) =
                    serde_urlencoded::from_str::<AccessInfo>(request.query_string())
                {
                    Arc::new(info.access_token.to_string())
                } else {
                    peek_body_token(&mut request).await
                }
            } else {
                EMPTY_ARC_STRING.clone()
            };
            let (pass, visitor_forbidden) = if !enable_auth || !is_check_path {
                (true, false)
            } else if token.is_empty() {
                (false, false)
            } else if let Ok(Some(session)) = get_user_session(
                &app_share_data,
                CacheKey::new(CacheType::ApiTokenSession, token.clone()),
            )
            .await
            {
                // Check if visitor is attempting a write operation (POST/PUT/DELETE)
                let method = request.method().as_str();
                let is_write = matches!(method, "POST" | "PUT" | "DELETE");
                let is_visitor_only = is_only_visitor_role(&session.roles);
                if is_write && is_visitor_only {
                    (false, true)
                } else {
                    request.extensions_mut().insert(session);
                    (true, false)
                }
            } else {
                (false, false)
            };
            //log::info!( "open api auth: {}|{}|{}|{}|{}|{}", &token, open_auth, is_check_path, pass, request.path(), request.query_string() );
            if pass {
                let res = service.call(request);
                // forwarded responses map to "left" body
                //record_req_metrics(&app_share_data.metrics_manager,duration,false);
                //res.await.map(ServiceResponse::map_into_left_body)
                res.await.map(move |item| {
                    let success = item.response().status().as_u16() < 400;
                    let duration = SystemTime::now()
                        .duration_since(start)
                        .unwrap_or_default()
                        .as_secs_f64();
                    if !ignore_metrics {
                        record_req_metrics(&app_share_data.metrics_manager, duration, success);
                    }
                    ServiceResponse::map_into_left_body(item)
                })
            } else {
                let forbidden_msg = if visitor_forbidden {
                    "visitor role has no write permission!"
                } else {
                    "unknown user!"
                };
                let body=format!("{{\"timestamp\":\"{}\",\"status\":403,\"error\":\"Forbidden\",\"message\":\"{}\",\"path\":\"{}\"}}"
                                 ,datetime_utils::get_now_timestamp_str(offset),forbidden_msg,request.path());
                let response = HttpResponse::Forbidden()
                    .insert_header(("Content-Type", "application/json;charset=UTF-8"))
                    .body(body)
                    //.json(ApiResult::<()>::error("AUTH_ERROR".to_owned(), None))
                    .map_into_right_body();
                let (http_request, _pl) = request.into_parts();
                let res = ServiceResponse::new(http_request, response);
                let duration = SystemTime::now()
                    .duration_since(start)
                    .unwrap_or_default()
                    .as_secs_f64();
                record_req_metrics(&app_share_data.metrics_manager, duration, false);
                Ok(res)
            }
        })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessInfo<'a> {
    pub access_token: &'a str,
}

async fn peek_body_token(request: &mut ServiceRequest) -> Arc<String> {
    let mut result = EMPTY_ARC_STRING.clone();
    if request.method().as_str() == "GET" {
        return result;
    }
    if let Ok(p) = request.extract::<web::Payload>().await {
        if let Ok(v) = p.to_bytes().await {
            //let body_str = String::from_utf8_lossy(v.as_ref());
            //log::info!("body info: {}",body_str.as_ref());
            if let Ok(info) = serde_urlencoded::from_bytes::<AccessInfo>(v.as_ref()) {
                result = Arc::new(info.access_token.to_string())
            }
            request.set_payload(bytes_to_payload(v));
        }
    };
    result
}

fn bytes_to_payload(buf: web::Bytes) -> dev::Payload {
    let (_, mut pl) = actix_http::h1::Payload::create(true);
    pl.unread_data(buf);
    dev::Payload::from(pl)
}

async fn get_user_session(
    app_share_data: &Arc<AppShareData>,
    key: CacheKey,
) -> anyhow::Result<Option<Arc<TokenSession>>> {
    /*
    let req = CacheManagerReq::Get(key);
    match app_share_data.cache_manager.send(req).await?? {
        CacheManagerResult::Value(CacheValue::ApiTokenSession(session)) => Ok(Some(session)),
        _ => Ok(None),
    }
    */
    let req = crate::cache::actor_model::CacheManagerLocalReq::Get(key);
    if let CacheManagerRaftResult::Value(crate::cache::model::CacheValue::ApiTokenSession(
        session,
    )) = app_share_data
        .direct_cache_manager
        .send(req.clone())
        .await??
    {
        Ok(Some(session))
    } else {
        //再尝试从raft主节点中获取
        if let RouterResponse::CacheQueryResult {
            result:
                CacheManagerRaftResult::Value(crate::cache::model::CacheValue::ApiTokenSession(v)),
        } = app_share_data
            .raft_request_route
            .request_from_main(app_share_data, RouterRequest::CacheQuery { req })
            .await?
        {
            Ok(Some(v))
        } else {
            Ok(None)
        }
    }
}

/// Check if the user only has the visitor role (no higher privilege role).
fn is_only_visitor_role(roles: &[Arc<String>]) -> bool {
    if roles.is_empty() {
        return false;
    }
    for role in roles {
        let user_role = UserRole::new(role.as_str());
        match user_role {
            UserRole::Manager | UserRole::Developer => return false,
            _ => {}
        }
    }
    true
}

fn record_req_metrics(metrics_manager: &Addr<MetricsManager>, duration: f64, _success: bool) {
    metrics_manager.do_send(MetricsRequest::BatchRecord(vec![
        MetricsItem::new(
            MetricsKey::HttpRequestHandleRtHistogram,
            MetricsRecord::HistogramRecord(duration as f32 * 1000f32),
        ),
        MetricsItem::new(
            MetricsKey::HttpRequestTotalCount,
            MetricsRecord::CounterInc(1),
        ),
    ]));
}
