use std::sync::Arc;

use actix_http::HttpMessage;
use actix_web::{
    web::{self, Data},
    HttpRequest, HttpResponse, Responder,
};
use serde::{Deserialize, Serialize};

use crate::{
    common::{
        appdata::AppShareData,
        model::{ApiResult, UserSession},
    },
    user::{model::UserDto, UserManagerReq, UserManagerResult},
};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetPasswordParam {
    pub old_password: String,
    pub new_password: String,
}

pub async fn get_user_info(req: HttpRequest) -> actix_web::Result<impl Responder> {
    if let Some(session) = req.extensions().get::<Arc<UserSession>>() {
        Ok(HttpResponse::Ok().json(ApiResult::success(Some(session.clone()))))
    } else {
        Ok(HttpResponse::Ok().json(ApiResult::<()>::error(
            "NOT_FOUND_USER_SESSION".to_owned(),
            None,
        )))
    }
}

pub async fn reset_password(
    req: HttpRequest,
    app: Data<Arc<AppShareData>>,
    web::Form(param): web::Form<ResetPasswordParam>,
) -> actix_web::Result<impl Responder> {
    if let Some(session) = req.extensions().get::<Arc<UserSession>>() {
        let username = Arc::new(session.username.to_string());
        let msg = UserManagerReq::CheckUser {
            name: username.clone(),
            password: param.old_password,
        };
        if let Ok(Ok(v)) = app.user_manager.send(msg).await {
            match v {
                UserManagerResult::CheckUserResult(valid, _user) => {
                    if valid {
                        let msg = UserManagerReq::UpdateUser {
                            user: UserDto {
                                username: username,
                                password: Some(param.new_password),
                                ..Default::default()
                            },
                        };
                        if let Ok(Ok(_r)) = app.user_manager.send(msg).await {
                            return Ok(HttpResponse::Ok().json(ApiResult::success(Some(true))));
                        }
                    }
                }
                _ => {
                    return Ok(HttpResponse::Ok().json(ApiResult::<()>::error(
                        "OLD_PASSWORD_INVALID".to_owned(),
                        None,
                    )))
                }
            }
        }
        Ok(HttpResponse::Ok().json(ApiResult::<()>::error("SYSTEM_ERROR".to_owned(), None)))
    } else {
        Ok(HttpResponse::Ok().json(ApiResult::<()>::error(
            "NOT_FOUND_USER_SESSION".to_owned(),
            None,
        )))
    }
}
