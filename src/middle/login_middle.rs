use std::future::{ready, Ready};
use std::sync::Arc;

use actix::Addr;
use actix_http::StatusCode;
use actix_web::{
    body::EitherBody,
    dev::{self, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpResponse,
};
use futures_util::future::LocalBoxFuture;

use crate::common::appdata::AppShareData;
use crate::common::model::{ApiResult, UserSession};
use crate::raft::cache::model::{CacheKey, CacheType, CacheValue};
use crate::raft::cache::{CacheManager, CacheManagerReq, CacheManagerResult};

lazy_static::lazy_static! {
    pub static ref ignore_check_login: Vec<&'static str> = vec!["/login", "/api/login/login"];
}

#[derive(Clone)]
pub struct CheckLogin {
    app_share_data: Arc<AppShareData>,
}

impl CheckLogin {
    pub fn new(app_share_data: Arc<AppShareData>) -> Self {
        Self { app_share_data }
    }
}

impl<S, B> Transform<S, ServiceRequest> for CheckLogin
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = CheckLoginMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(CheckLoginMiddleware {
            service,
            app_share_data: self.app_share_data.clone(),
        }))
    }
}

#[derive(Clone)]
pub struct CheckLoginMiddleware<S> {
    service: S,
    app_share_data: Arc<AppShareData>,
}

impl<S, B> Service<ServiceRequest> for CheckLoginMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    dev::forward_ready!(service);

    fn call(&self, request: ServiceRequest) -> Self::Future {
        let path = request.path();
        let is_page = true;
        let is_check_path = !ignore_check_login.contains(&path);
        let token = if let Some(ck) = request.cookie("token") {
            ck.value().to_owned()
        } else {
            if let Some(v) = request.headers().get("Token") {
                v.to_str().unwrap_or_default().to_owned()
            } else {
                "".to_owned()
            }
        };
        let token = Arc::new(token);
        let cache_manager = self.app_share_data.cache_manager.clone();
        //request.parts()
        let (http_request, _pl) = request.parts();
        let http_request = http_request.to_owned();
        let res = self.service.call(request);
        //let service = self.service.clone();
        Box::pin(async move {
            let mut is_login = false;
            if is_check_path {
                is_login = if token.is_empty() {
                    false
                } else {
                    if let Ok(Some(_session)) = get_user_session(
                        &cache_manager,
                        CacheManagerReq::Get(CacheKey::new(CacheType::UserSession, token)),
                    )
                    .await
                    {
                        true
                    } else {
                        false
                    }
                };
            }
            if is_login {
                //let res = service.call(request);
                // forwarded responses map to "left" body
                res.await.map(ServiceResponse::map_into_left_body)
            } else {
                let response = if is_page {
                    HttpResponse::Ok()
                        .insert_header(("Location", "/login"))
                        .status(StatusCode::MOVED_PERMANENTLY)
                        .finish()
                        .map_into_right_body()
                } else {
                    let mut res = ApiResult::<bool>::default();
                    res.code = Some("NO_LOGIN".to_owned());
                    HttpResponse::Ok().json(res).map_into_right_body()
                };
                //let (request, _pl) = request.into_parts();
                let res = ServiceResponse::new(http_request, response);
                Ok(res)
            }
        })
    }
}

async fn get_user_session(
    cache_manager: &Addr<CacheManager>,
    req: CacheManagerReq,
) -> anyhow::Result<Option<UserSession>> {
    match cache_manager.send(req).await?? {
        CacheManagerResult::None => Ok(None),
        CacheManagerResult::Value(v) => match v {
            CacheValue::UserSession(session) => Ok(Some(session)),
            _ => Ok(None),
        },
    }
}
