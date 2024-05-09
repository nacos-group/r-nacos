use crate::common::appdata::AppShareData;
use crate::common::crypto_utils;
use crate::common::model::{ApiResult, UserSession};
pub use crate::console::login_api::{gen_captcha, logout};
use crate::console::model::login_model::LoginParam;
use crate::raft::cache::model::{CacheKey, CacheType, CacheValue};
use crate::raft::cache::{CacheLimiterReq, CacheManagerReq, CacheManagerResult};
use crate::user::{UserManagerReq, UserManagerResult};
use actix_web::cookie::Cookie;
use actix_web::web::Data;
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use std::sync::Arc;

pub async fn login(
    request: HttpRequest,
    app: Data<Arc<AppShareData>>,
    web::Json(param): web::Json<LoginParam>,
) -> actix_web::Result<impl Responder> {
    //校验验证码
    let captcha_token = if let Some(ck) = request.cookie("captcha_token") {
        ck.value().to_owned()
    } else {
        return Ok(HttpResponse::Ok().json(ApiResult::<()>::error(
            "CAPTCHA_CHECK_ERROR".to_owned(),
            Some("captcha token is empty".to_owned()),
        )));
    };
    let captcha_code = param.captcha.to_uppercase();
    let cache_req = CacheManagerReq::Get(CacheKey::new(
        CacheType::String,
        Arc::new(format!("Captcha_{}", &captcha_token)),
    ));
    let captcha_check_result = if let Ok(Ok(CacheManagerResult::Value(CacheValue::String(v)))) =
        app.cache_manager.send(cache_req).await
    {
        &captcha_code == v.as_ref()
    } else {
        false
    };
    if !captcha_check_result {
        return Ok(HttpResponse::Ok()
            .cookie(
                Cookie::build("captcha_token", "")
                    .path("/")
                    .http_only(true)
                    .finish(),
            )
            .json(ApiResult::<()>::error(
                "CAPTCHA_CHECK_ERROR".to_owned(),
                Some("CAPTCHA_CHECK_ERROR".to_owned()),
            )));
    }
    let limit_key = Arc::new(format!("USER_L#{}", &param.username));
    let limit_req = CacheLimiterReq::Hour {
        key: limit_key.clone(),
        limit: app.sys_config.console_login_one_hour_limit as i32,
    };
    //登录前先判断是否登陆准入
    if let Ok(CacheManagerResult::Limiter(acquire_result)) =
        app.raft_cache_route.request_limiter(limit_req).await
    {
        if !acquire_result {
            return Ok(HttpResponse::Ok().json(ApiResult::<()>::error(
                "LOGIN_LIMITE_ERROR".to_owned(),
                Some("Frequent login, please try again later".to_owned()),
            )));
        }
    } else {
        return Ok(HttpResponse::Ok().json(ApiResult::<()>::error("SYSTEM_ERROR".to_owned(), None)));
    }
    let password = match decode_password(&param.password, &captcha_token) {
        Ok(v) => v,
        Err(e) => {
            log::error!("decode_password error:{}", e);
            return Ok(HttpResponse::Ok().json(ApiResult::<()>::error(
                "SYSTEM_ERROR".to_owned(),
                Some("decode_password error".to_owned()),
            )));
        }
    };
    let msg = UserManagerReq::CheckUser {
        name: param.username,
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
            let session = Arc::new(UserSession {
                username: user.username,
                nickname: user.nickname,
                roles: user.roles.unwrap_or_default(),
                extend_infos: user.extend_info.unwrap_or_default(),
            });
            let cache_req = CacheManagerReq::Set {
                key: CacheKey::new(CacheType::UserSession, token.clone()),
                value: CacheValue::UserSession(session),
                ttl: app.sys_config.console_login_timeout,
            };
            app.cache_manager.do_send(cache_req);
            //登录成功后清除登陆限流计数
            let clear_limit_req =
                CacheManagerReq::Remove(CacheKey::new(CacheType::String, limit_key));
            app.cache_manager.do_send(clear_limit_req);
            return Ok(HttpResponse::Ok()
                .cookie(
                    Cookie::build("token", token.as_str())
                        .path("/")
                        .http_only(true)
                        .finish(),
                )
                .cookie(
                    Cookie::build("captcha_token", "")
                        .path("/")
                        .http_only(true)
                        .finish(),
                )
                .json(ApiResult::success(Some(valid))));
        } else {
            return Ok(HttpResponse::Ok()
                .json(ApiResult::<()>::error("USER_CHECK_ERROR".to_owned(), None)));
        }
    }
    Ok(HttpResponse::Ok().json(ApiResult::<()>::error("SYSTEM_ERROR".to_owned(), None)))
}

fn decode_password(password: &str, captcha_token: &str) -> anyhow::Result<String> {
    let password_data = crypto_utils::decode_base64(password)?;
    let password = String::from_utf8(crypto_utils::decrypt_aes128(
        &captcha_token[0..16],
        &captcha_token[16..32],
        &password_data,
    )?)?;
    Ok(password)
}
