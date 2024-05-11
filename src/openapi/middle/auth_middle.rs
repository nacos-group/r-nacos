use crate::common::appdata::AppShareData;
use crate::common::model::{ApiResult, TokenSession};
use crate::raft::cache::model::{CacheKey, CacheType, CacheValue};
use crate::raft::cache::{CacheManager, CacheManagerReq, CacheManagerResult};
use actix::Addr;
use actix_http::body::EitherBody;
use actix_http::HttpMessage;
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::{dev, Error, HttpResponse};
use futures_util::future::LocalBoxFuture;
use regex::Regex;
use std::future::{ready, Ready};
use std::sync::Arc;

pub(crate) const AUTHORIZATION_HEADER: &str = "Authorization";

lazy_static::lazy_static! {
    pub static ref IGNORE_PATH: Vec<&'static str> = vec![
        "/nacos/v1/auth/login", "/nacos/v1/auth/users/login",
    ];
    pub static ref API_PATH: Regex = Regex::new(r"(?i)/nacos/.*").unwrap();
    pub static ref PARM_AUTH_TOKEN: Regex = Regex::new(r"accessToken=(\w*)").unwrap();
    pub static ref EMPTY_TOKEN: Arc<String> = Arc::new("".to_owned());
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

    fn call(&self, request: ServiceRequest) -> Self::Future {
        let open_auth = self.app_share_data.sys_config.openapi_open_auth;
        let path = request.path();
        let is_check_path = if open_auth {
            API_PATH.is_match(path) && !IGNORE_PATH.contains(&path)
        } else {
            true
        };
        let token = if open_auth && is_check_path {
            if let Some(v) = request.headers().get(AUTHORIZATION_HEADER) {
                Arc::new(v.to_str().unwrap_or_default().to_owned())
            } else if let Some(c) = PARM_AUTH_TOKEN.captures(request.query_string()) {
                c.get(1)
                    .map_or(EMPTY_TOKEN.clone(), |m| Arc::new(m.as_str().to_owned()))
            } else {
                EMPTY_TOKEN.clone()
            }
        } else {
            EMPTY_TOKEN.clone()
        };
        let cache_manager = self.app_share_data.cache_manager.clone();
        let service = self.service.clone();
        Box::pin(async move {
            let pass = if !open_auth || !is_check_path {
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
                let response = HttpResponse::BadGateway()
                    //.body("AUTH_ERROR")
                    .json(ApiResult::<()>::error("AUTH_ERROR".to_owned(), None))
                    .map_into_right_body();
                let (http_request, _pl) = request.into_parts();
                let res = ServiceResponse::new(http_request, response);
                Ok(res)
            }
        })
    }
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
