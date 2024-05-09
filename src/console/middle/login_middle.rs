use std::collections::HashMap;
use std::future::{ready, Ready};
use std::sync::Arc;

use actix::Addr;
use actix_http::{HttpMessage, StatusCode};
use actix_web::{
    body::EitherBody,
    dev::{self, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpResponse,
};
use futures_util::future::LocalBoxFuture;
use regex::Regex;

use crate::common::appdata::AppShareData;
use crate::common::model::{ApiResultOld, UserSession};
use crate::raft::cache::model::{CacheKey, CacheType, CacheValue};
use crate::raft::cache::{CacheManager, CacheManagerReq, CacheManagerResult};
use crate::user::permission::UserRole;

lazy_static::lazy_static! {
    pub static ref IGNORE_CHECK_LOGIN: Vec<&'static str> = vec![
        "/rnacos/p/login", "/rnacos/404",
        "/rnacos/api/console/login/login", "/rnacos/api/console/login/captcha",
        "/rnacos/api/console/v2/login/login", "/rnacos/api/console/v2/login/captcha",
    ];
    pub static ref STATIC_FILE_PATH: Regex= Regex::new(r"(?i).*\.(js|css|png|jpg|jpeg|bmp|svg)").unwrap();
    pub static ref API_PATH: Regex = Regex::new(r"(?i)/(api|nacos)/.*").unwrap();
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
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
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
            service: Arc::new(service),
            app_share_data: self.app_share_data.clone(),
        }))
    }
}

#[derive(Clone)]
pub struct CheckLoginMiddleware<S> {
    service: Arc<S>,
    app_share_data: Arc<AppShareData>,
}

impl<S, B> Service<ServiceRequest> for CheckLoginMiddleware<S>
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
        let path = request.path();
        let is_check_path = !IGNORE_CHECK_LOGIN.contains(&path) && !STATIC_FILE_PATH.is_match(path);
        let is_page = !API_PATH.is_match(path);
        let token = if let Some(ck) = request.cookie("token") {
            ck.value().to_owned()
        } else if let Some(v) = request.headers().get("Token") {
            v.to_str().unwrap_or_default().to_owned()
        } else {
            "".to_owned()
        };
        let token = Arc::new(token);
        let cache_manager = self.app_share_data.cache_manager.clone();
        //request.parts()
        //let (http_request, _pl) = request.parts();
        //let http_request = http_request.to_owned();
        //let res = self.service.call(request);

        let service = self.service.clone();
        Box::pin(async move {
            let mut is_login = true;
            let mut user_has_permission = true;
            let path = request.path();
            let method = request.method().as_str();
            if is_check_path {
                is_login = if token.is_empty() {
                    false
                } else if let Ok(Some(session)) = get_user_session(
                    &cache_manager,
                    CacheManagerReq::Get(CacheKey::new(CacheType::UserSession, token.clone())),
                )
                .await
                {
                    user_has_permission =
                        UserRole::match_url_by_roles(&session.roles, path, method);
                    request.extensions_mut().insert(session);
                    true
                } else {
                    false
                };
            }
            //log::info!("token: {}|{}|{}|{}|{}|{}",&token,is_page,is_check_path,is_login,request.path(),request.query_string());
            if is_login {
                if user_has_permission {
                    let res = service.call(request);
                    // forwarded responses map to "left" body
                    res.await.map(ServiceResponse::map_into_left_body)
                } else {
                    //已登录没有权限
                    let response = if is_page {
                        //let move_url = format!("/nopermission?path={}", request.path());
                        let move_url = format!("/rnacos/nopermission?path={}", request.path());
                        HttpResponse::Ok()
                            .insert_header(("Location", move_url))
                            .status(StatusCode::FOUND)
                            .finish()
                            .map_into_right_body()
                    } else {
                        HttpResponse::Ok()
                            .insert_header(("No-Permission", "1"))
                            .json(ApiResultOld::<()>::error("NO_PERMISSION".to_owned(), None))
                            .map_into_right_body()
                    };
                    let (http_request, _pl) = request.into_parts();
                    let res = ServiceResponse::new(http_request, response);
                    Ok(res)
                }
            } else {
                //没有登录
                let response = if is_page {
                    let move_url =
                        if request.path() == "/rnacos/p/login" || request.path() == "/p/login" {
                            format!("{}?{}", request.path(), request.query_string())
                        } else {
                            let mut redirect_param = HashMap::new();
                            if !request.query_string().is_empty() {
                                redirect_param.insert(
                                    "redirect_url",
                                    format!("{}?{}", request.path(), request.query_string()),
                                );
                            } else {
                                redirect_param.insert("redirect_url", request.path().to_owned());
                            };
                            let redirect_param =
                                serde_urlencoded::to_string(&redirect_param).unwrap_or_default();
                            format!("/rnacos/p/login?{}", redirect_param)
                        };
                    HttpResponse::Ok()
                        .insert_header(("Location", move_url))
                        .status(StatusCode::FOUND)
                        .finish()
                        .map_into_right_body()
                } else {
                    HttpResponse::Ok()
                        .insert_header(("No-Login", "1"))
                        .json(ApiResultOld::<()>::error("NO_LOGIN".to_owned(), None))
                        .map_into_right_body()
                };
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
) -> anyhow::Result<Option<Arc<UserSession>>> {
    match cache_manager.send(req).await?? {
        CacheManagerResult::Value(CacheValue::UserSession(session)) => Ok(Some(session)),
        _ => Ok(None),
    }
}
