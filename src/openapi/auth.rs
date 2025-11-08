use crate::common::appdata::AppShareData;
use crate::common::model::TokenSession;
use crate::common::option_utils::OptionUtils;
use crate::merge_web_param_with_result;
use crate::raft::cache::model::{CacheKey, CacheType, CacheValue};
use crate::raft::cache::{CacheLimiterReq, CacheManagerReq, CacheManagerResult};
use crate::user::{UserManagerReq, UserManagerResult};
use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginParams {
    pub username: Option<String>,
    pub password: Option<String>,
}

impl LoginParams {
    pub fn merge(self, other: Self) -> Self {
        Self {
            username: OptionUtils::select(self.username, other.username),
            password: OptionUtils::select(self.password, other.password),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct LoginResult {
    pub access_token: Option<Arc<String>>,
    pub token_ttl: i64,
    pub global_admin: bool,
    pub username: Option<Arc<String>>,
}

const UNKNOWN_USER: &str = "unknown user!";

pub async fn login(
    app: web::Data<Arc<AppShareData>>,
    web::Query(param): web::Query<LoginParams>,
    payload: web::Payload,
) -> actix_web::Result<impl Responder> {
    let param = merge_web_param_with_result!(param, payload);
    match do_login(param, &app).await {
        Ok(v) => Ok(v),
        Err(e) => {
            if !app.sys_config.openapi_enable_auth {
                Ok(HttpResponse::Ok().body(format!(
                    "{{\"accessToken\":\"AUTH_DISABLED\",\"tokenTtl\":{},\"globalAdmin\":true}}",
                    app.sys_config.openapi_login_timeout
                )))
            } else {
                Ok(HttpResponse::Forbidden().body(e.to_string()))
            }
        }
    }
}

async fn do_login(
    param: LoginParams,
    app: &web::Data<Arc<AppShareData>>,
) -> anyhow::Result<HttpResponse> {
    let username = Arc::new(param.username.unwrap_or_default());
    let password = param.password.unwrap_or_default();
    let limit_key = Arc::new(format!("API_USER_L#{}", &username));
    let limit_req = CacheLimiterReq::Minutes {
        key: limit_key.clone(),
        limit: app.sys_config.openapi_login_one_minute_limit as i32,
    };
    //登录前先判断是否登陆准入
    if let Ok(CacheManagerResult::Limiter(acquire_result)) =
        app.raft_cache_route.request_limiter(limit_req).await
    {
        if !acquire_result {
            return Err(anyhow::anyhow!(
                "LOGIN_LIMITE_ERROR,Frequent login, please try again later"
            ));
        }
    } else {
        return Err(anyhow::anyhow!("SYSTEM_ERROR"));
    }
    let msg = UserManagerReq::CheckUser {
        name: username,
        password,
    };
    if let Ok(Ok(UserManagerResult::CheckUserResult(valid, user))) =
        app.user_manager.send(msg).await
    {
        if valid {
            //增加长度避免遍历
            let token = Arc::new(
                uuid::Uuid::new_v4().to_string().replace('-', "")
                    + &uuid::Uuid::new_v4().to_string().replace('-', ""),
            );
            let session = Arc::new(TokenSession {
                username: user.username.clone(),
                roles: user.roles.unwrap_or_default(),
                extend_infos: user.extend_info.unwrap_or_default(),
            });
            let cache_req = CacheManagerReq::Set {
                key: CacheKey::new(CacheType::ApiTokenSession, token.clone()),
                value: CacheValue::ApiTokenSession(session),
                ttl: app.sys_config.openapi_login_timeout,
            };
            app.cache_manager.do_send(cache_req);
            //登录成功后清除登陆限流计数
            let clear_limit_req =
                CacheManagerReq::Remove(CacheKey::new(CacheType::String, limit_key));
            app.cache_manager.do_send(clear_limit_req);
            let login_result = LoginResult {
                access_token: Some(token),
                token_ttl: app.sys_config.openapi_login_timeout as i64,
                global_admin: false,
                username: Some(user.username),
            };
            return Ok(HttpResponse::Ok().json(login_result));
        } else {
            return Err(anyhow::anyhow!(UNKNOWN_USER));
        }
    }
    Err(anyhow::anyhow!(UNKNOWN_USER))
}

pub(crate) async fn mock_token() -> impl Responder {
    "{\"accessToken\":\"mock_token\",\"tokenTtl\":18000,\"globalAdmin\":true}"
}

pub fn login_config(config: &mut web::ServiceConfig) {
    config
        .service(
            web::resource("/nacos/v3/auth/user/login")
                .route(web::post().to(login))
                .route(web::get().to(login)),
        )
        .service(
            web::resource("/nacos/v1/auth/users/login")
                .route(web::post().to(login))
                .route(web::get().to(login)),
        )
        .service(
            web::resource("/nacos/v1/auth/login")
                .route(web::post().to(login))
                .route(web::get().to(login)),
        );
}
