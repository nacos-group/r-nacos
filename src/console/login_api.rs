use std::collections::HashMap;
use std::sync::Arc;

use super::model::login_model::{LoginParam, LoginToken};
use crate::ldap::model::actor_model::{LdapMsgReq, LdapMsgResult};
use crate::ldap::model::LdapUserParam;
use crate::{
    common::{
        appdata::AppShareData,
        crypto_utils,
        model::{ApiResult, UserSession},
    },
    now_second_i32,
    raft::cache::{
        model::{CacheKey, CacheType, CacheValue},
        CacheLimiterReq, CacheManagerReq, CacheManagerResult,
    },
    user::{UserManagerReq, UserManagerResult},
};
use actix_web::http::header;
use actix_web::{
    cookie::Cookie,
    web::{self, Data},
    HttpRequest, HttpResponse, Responder,
};
use captcha::filters::{Grid, Noise};
use captcha::Captcha;

pub async fn login(
    request: HttpRequest,
    app: Data<Arc<AppShareData>>,
    web::Form(param): web::Form<LoginParam>,
) -> HttpResponse {
    let captcha_token = if let Some(ck) = request.cookie("captcha_token") {
        ck.value().to_owned()
    } else {
        String::new()
    };
    if app.sys_config.console_captcha_enable {
        //校验验证码
        if let Some(value) = check_captcha(
            &app,
            param.captcha.clone().unwrap_or_default().to_uppercase(),
            &captcha_token,
        )
        .await
        {
            return value;
        }
    }

    let limit_key = Arc::new(format!("USER_L#{}", &param.username));
    if let Some(value) = login_limit(&app, &limit_key).await {
        return value;
    }
    let password = match decode_password(&param.password, &captcha_token) {
        Ok(v) => v,
        Err(e) => {
            log::error!("decode_password error:{}", e);
            return HttpResponse::Ok().json(ApiResult::<()>::error(
                "SYSTEM_ERROR".to_owned(),
                Some("decode_password error".to_owned()),
            ));
        }
    };
    let mut session = None;
    let msg = UserManagerReq::CheckUser {
        name: param.username.clone(),
        password: password.clone(),
    };
    let mut error_code = "USER_CHECK_ERROR".to_owned();
    if let Ok(Ok(res)) = app.user_manager.send(msg).await {
        if let UserManagerResult::CheckUserResult(valid, user) = res {
            if valid {
                session = Some(Arc::new(UserSession {
                    username: user.username,
                    nickname: user.nickname,
                    roles: user.roles.unwrap_or_default(),
                    extend_infos: user.extend_info.unwrap_or_default(),
                    namespace_privilege: user.namespace_privilege,
                    refresh_time: now_second_i32() as u32,
                }));
            }
        }
        if !app.sys_config.ldap_enable && session.is_none() {
            return HttpResponse::Ok().json(ApiResult::<()>::error(error_code, None));
        }
    } else {
        error_code = "SYSTEM_ERROR".to_owned();
    }
    if session.is_none() && app.sys_config.ldap_enable {
        match ldap_login(&app, param.clone(), password).await {
            Ok(v) => {
                session = v;
            }
            Err(e) => {
                return HttpResponse::Ok()
                    .json(ApiResult::<()>::error(error_code, Some(e.to_string())));
            }
        }
    }
    if let Some(session) = session {
        if let Some(value) = apply_session(app, limit_key, session) {
            return value;
        }
    }
    HttpResponse::Ok().json(ApiResult::<()>::error(error_code, None))
}

fn apply_session(
    app: Data<Arc<AppShareData>>,
    limit_key: Arc<String>,
    session: Arc<UserSession>,
) -> Option<HttpResponse> {
    //增加长度避免遍历
    let token = Arc::new(
        uuid::Uuid::new_v4().to_string().replace('-', "")
            + &uuid::Uuid::new_v4().to_string().replace('-', ""),
    );
    let cache_req = CacheManagerReq::Set {
        key: CacheKey::new(CacheType::UserSession, token.clone()),
        value: CacheValue::UserSession(session),
        ttl: app.sys_config.console_login_timeout,
    };
    app.cache_manager.do_send(cache_req);
    //登录成功后清除登陆限流计数
    let clear_limit_req = CacheManagerReq::Remove(CacheKey::new(CacheType::String, limit_key));
    app.cache_manager.do_send(clear_limit_req);
    let login_token = LoginToken {
        token: token.to_string(),
    };
    Some(
        HttpResponse::Ok()
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
            .insert_header(header::ContentType(mime::APPLICATION_JSON))
            .json(ApiResult::success(Some(login_token))),
    )
}

async fn ldap_login(
    app: &Data<Arc<AppShareData>>,
    param: LoginParam,
    password: String,
) -> anyhow::Result<Option<Arc<UserSession>>> {
    let res = app
        .ldap_manager
        .send(LdapMsgReq::Bind(LdapUserParam {
            user_name: param.username.as_ref().to_owned(),
            password,
            query_meta: true,
        }))
        .await??;
    let v = if let LdapMsgResult::UserMeta(meta) = res {
        Some(Arc::new(UserSession {
            username: param.username.clone(),
            nickname: Some(meta.user_name),
            roles: vec![meta.role],
            namespace_privilege: meta.namespace_privilege,
            extend_infos: HashMap::default(),
            refresh_time: now_second_i32() as u32,
        }))
    } else {
        None
    };
    Ok(v)
}

async fn login_limit(
    app: &Data<Arc<AppShareData>>,
    limit_key: &Arc<String>,
) -> Option<HttpResponse> {
    let limit_req = CacheLimiterReq::Hour {
        key: limit_key.clone(),
        limit: app.sys_config.console_login_one_hour_limit as i32,
    };
    //登录前先判断是否登陆准入
    if let Ok(CacheManagerResult::Limiter(acquire_result)) =
        app.raft_cache_route.request_limiter(limit_req).await
    {
        if !acquire_result {
            return Some(HttpResponse::Ok().json(ApiResult::<()>::error(
                "LOGIN_LIMITE_ERROR".to_owned(),
                Some("Frequent login, please try again later".to_owned()),
            )));
        }
    } else {
        return Some(
            HttpResponse::Ok().json(ApiResult::<()>::error("SYSTEM_ERROR".to_owned(), None)),
        );
    }
    None
}

async fn check_captcha(
    app: &Data<Arc<AppShareData>>,
    captcha_code: String,
    captcha_token: &String,
) -> Option<HttpResponse> {
    if captcha_token.is_empty() {
        return Some(HttpResponse::Ok().json(ApiResult::<()>::error(
            "CAPTCHA_CHECK_ERROR".to_owned(),
            Some("captcha token is empty".to_owned()),
        )));
    }
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
        return Some(
            HttpResponse::Ok()
                .cookie(
                    Cookie::build("captcha_token", "")
                        .path("/")
                        .http_only(true)
                        .finish(),
                )
                .json(ApiResult::<()>::error(
                    "CAPTCHA_CHECK_ERROR".to_owned(),
                    Some("CAPTCHA_CHECK_ERROR".to_owned()),
                )),
        );
    }
    None
}

fn decode_password(password: &str, captcha_token: &str) -> anyhow::Result<String> {
    let password_data = crypto_utils::decode_base64(password)?;
    if captcha_token.is_empty() {
        let password = String::from_utf8(password_data)?;
        Ok(password)
    } else {
        let password = String::from_utf8(crypto_utils::decrypt_aes128(
            &captcha_token[0..16],
            &captcha_token[16..32],
            &password_data,
        )?)?;
        Ok(password)
    }
}

const WIDTH: u32 = 220;
const HEIGHT: u32 = 120;

pub async fn gen_captcha(app: Data<Arc<AppShareData>>) -> actix_web::Result<impl Responder> {
    let token = uuid::Uuid::new_v4().to_string().replace('-', "");
    let captcha_cookie = Cookie::build("captcha_token", token.as_str())
        .path("/")
        .http_only(true)
        .finish();
    let captcha_header = ("Captcha-Token", token.as_str());

    // 如果验证码功能被禁用，data 为 null
    if !app.sys_config.console_captcha_enable {
        return Ok(HttpResponse::Ok()
            .cookie(captcha_cookie)
            .insert_header(captcha_header)
            .json(ApiResult::<String>::success(None)));
    }

    //let obj = gen(Difficulty::Easy);
    let mut obj = Captcha::new();
    obj.add_chars(4)
        .apply_filter(Noise::new(0.1))
        .apply_filter(Grid::new(8, 8))
        .view(WIDTH, HEIGHT);

    let code: String = obj.chars().iter().collect::<String>().to_uppercase();
    let code = Arc::new(code);

    let img = obj.as_base64().unwrap_or_default();
    //log::info!("gen_captcha code:{}", &code);
    let cache_req = CacheManagerReq::Set {
        key: CacheKey::new(CacheType::String, Arc::new(format!("Captcha_{}", &token))),
        value: CacheValue::String(code),
        ttl: 300,
    };
    app.cache_manager.send(cache_req).await.ok();
    Ok(HttpResponse::Ok()
        .cookie(captcha_cookie)
        .insert_header(captcha_header)
        .json(ApiResult::success(Some(img))))
}

pub async fn logout(
    request: HttpRequest,
    app: Data<Arc<AppShareData>>,
) -> actix_web::Result<impl Responder> {
    let token = if let Some(ck) = request.cookie("token") {
        ck.value().to_owned()
    } else if let Some(v) = request.headers().get("Token") {
        v.to_str().unwrap_or_default().to_owned()
    } else {
        "".to_owned()
    };
    let token = Arc::new(token);
    let cache_req = CacheManagerReq::Remove(CacheKey::new(CacheType::UserSession, token));
    app.cache_manager.do_send(cache_req);
    Ok(HttpResponse::Ok()
        .cookie(
            Cookie::build("token", "")
                .path("/")
                .http_only(true)
                .finish(),
        )
        .json(ApiResult::success(Some(true))))
}
