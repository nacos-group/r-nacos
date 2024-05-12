use crate::common::appdata::AppShareData;
use crate::common::constant::{AUTHORIZATION_HEADER, EMPTY_ARC_STRING};
use crate::common::datetime_utils;
use crate::common::model::TokenSession;
use crate::raft::cache::model::{CacheKey, CacheType, CacheValue};
use crate::raft::cache::{CacheManager, CacheManagerReq, CacheManagerResult};
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

lazy_static::lazy_static! {
    pub static ref IGNORE_PATH: Vec<&'static str> = vec![
        "/nacos/v1/auth/login", "/nacos/v1/auth/users/login",
    ];
    pub static ref API_PATH: Regex = Regex::new(r"(?i)/nacos/.*").unwrap();
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
        let mut request = req;
        let enable_auth = self.app_share_data.sys_config.openapi_enable_auth;
        let path = request.path();
        let is_check_path = if enable_auth {
            API_PATH.is_match(path) && !IGNORE_PATH.contains(&path)
        } else {
            true
        };

        let cache_manager = self.app_share_data.cache_manager.clone();
        let offset = self.app_share_data.timezone_offset.clone();
        let service = self.service.clone();
        Box::pin(async move {
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
            let pass = if !enable_auth || !is_check_path {
                true
            } else if token.is_empty() {
                false
            } else if let Ok(Some(session)) = get_user_session(
                &cache_manager,
                CacheManagerReq::Get(CacheKey::new(CacheType::ApiTokenSession, token.clone())),
            )
            .await
            {
                request.extensions_mut().insert(session);
                true
            } else {
                false
            };
            //log::info!( "open api auth: {}|{}|{}|{}|{}|{}", &token, open_auth, is_check_path, pass, request.path(), request.query_string() );
            if pass {
                let res = service.call(request);
                // forwarded responses map to "left" body
                res.await.map(ServiceResponse::map_into_left_body)
            } else {
                //没有登录
                let body=format!("{{\"timestamp\":\"{}\",\"status\":403,\"error\":\"Forbidden\",\"message\":\"unknown user!\",\"path\":\"{}\"}}"
                                 ,datetime_utils::get_now_timestamp_str(&offset),request.path());
                let response = HttpResponse::Forbidden()
                    .insert_header(("Content-Type", "application/json;charset=UTF-8"))
                    .body(body)
                    //.json(ApiResult::<()>::error("AUTH_ERROR".to_owned(), None))
                    .map_into_right_body();
                let (http_request, _pl) = request.into_parts();
                let res = ServiceResponse::new(http_request, response);
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
            request.set_payload(bytes_to_payload(v.into()));
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
    cache_manager: &Addr<CacheManager>,
    req: CacheManagerReq,
) -> anyhow::Result<Option<Arc<TokenSession>>> {
    match cache_manager.send(req).await?? {
        CacheManagerResult::Value(CacheValue::ApiTokenSession(session)) => Ok(Some(session)),
        _ => Ok(None),
    }
}
