use std::sync::Arc;

use actix_web::{
    cookie::Cookie,
    web::{self, Data},
    HttpResponse, Responder,
};

use crate::{
    common::{
        appdata::AppShareData,
        model::{ApiResult, UserSession},
    },
    raft::cache::{
        model::{CacheKey, CacheType, CacheValue},
        CacheManagerReq,
    },
    user::{UserManagerReq, UserManagerResult},
};

use super::model::login_model::LoginParam;

pub async fn login(
    app: Data<Arc<AppShareData>>,
    web::Form(param): web::Form<LoginParam>,
) -> actix_web::Result<impl Responder> {
    let msg = UserManagerReq::CheckUser {
        name: param.username,
        password: param.password,
    };
    if let Ok(Ok(v)) = app.user_manager.send(msg).await {
        match v {
            UserManagerResult::CheckUserResult(valid, user) => {
                if valid {
                    //增加长度避免遍历
                    let token = Arc::new(
                        uuid::Uuid::new_v4().to_string().replace("-", "")
                            + &uuid::Uuid::new_v4().to_string().replace("-", ""),
                    );
                    let session = Arc::new(UserSession {
                        username: user.username,
                        nickname: user.nickname,
                        ..Default::default()
                    });
                    let cache_req = CacheManagerReq::Set {
                        key: CacheKey::new(CacheType::UserSession, token.clone()),
                        value: CacheValue::UserSession(session),
                        ttl: app.sys_config.console_login_timeout,
                    };
                    app.cache_manager.do_send(cache_req);
                    return Ok(HttpResponse::Ok()
                        .cookie(
                            Cookie::build("token", token.as_str())
                                .path("/")
                                .http_only(true)
                                .finish(),
                        )
                        .json(ApiResult::success(Some(valid))));
                }
            }
            _ => {}
        };
    }
    Ok(HttpResponse::Ok().json(ApiResult::<()>::error("SYSTEM_ERROR".to_owned(), None)))
}
