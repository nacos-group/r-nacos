use crate::common::appdata::AppShareData;
use crate::common::model::{ApiResult, PageResult, UserSession};
use crate::console::model::user_model::{UpdateUserInfoParam, UserPageParams};
use crate::user::{UserManagerReq, UserManagerResult};
use actix_http::HttpMessage;
use actix_web::web::Data;
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use std::sync::Arc;

use crate::console::user_api::ResetPasswordParam;
pub use crate::console::user_api::{get_user_info, get_user_web_resources};
use crate::user::model::UserDto;

pub async fn reset_password(
    req: HttpRequest,
    app: Data<Arc<AppShareData>>,
    web::Json(param): web::Json<ResetPasswordParam>,
) -> actix_web::Result<impl Responder> {
    let (msg, username) = if let Some(session) = req.extensions().get::<Arc<UserSession>>() {
        let username = Arc::new(session.username.to_string());
        (
            UserManagerReq::CheckUser {
                name: username.clone(),
                password: param.old_password,
            },
            username,
        )
    } else {
        return Ok(HttpResponse::Ok().json(ApiResult::<()>::error(
            "NOT_FOUND_USER_SESSION".to_owned(),
            None,
        )));
    };
    if let Ok(Ok(v)) = app.user_manager.send(msg).await {
        match v {
            UserManagerResult::CheckUserResult(valid, _user) => {
                if valid {
                    let msg = UserManagerReq::UpdateUser {
                        user: UserDto {
                            username,
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
}

pub async fn get_user_page_list(
    app: Data<Arc<AppShareData>>,
    web::Query(param): web::Query<UserPageParams>,
) -> actix_web::Result<impl Responder> {
    let (limit, offset) = param.get_limit_info();
    let msg = UserManagerReq::QueryPageList {
        like_username: param.like_username,
        offset: Some(offset as i64),
        limit: Some(limit as i64),
        is_rev: param.is_rev.unwrap_or_default(),
    };
    match app.user_manager.send(msg).await.unwrap().unwrap() {
        UserManagerResult::UserPageResult(total_count, list) => {
            Ok(HttpResponse::Ok().json(ApiResult::success(Some(PageResult { total_count, list }))))
        }
        _ => Ok(HttpResponse::Ok().json(ApiResult::<()>::error(
            "NOT_FOUND_USER".to_owned(),
            Some("result type is error".to_owned()),
        ))),
    }
}

pub async fn add_user(
    app: Data<Arc<AppShareData>>,
    web::Json(user_param): web::Json<UpdateUserInfoParam>,
) -> actix_web::Result<impl Responder> {
    let user: UserDto = user_param.into();
    if user.roles.is_none() {
        return Ok(HttpResponse::Ok().json(ApiResult::<()>::error(
            "USER_ROLE_IS_EMPTY".to_string(),
            Some("user roles is empty".to_owned()),
        )));
    }
    let msg = UserManagerReq::AddUser {
        user: UserDto {
            username: user.username,
            password: Some(user.password.unwrap_or_default()),
            gmt_create: None,
            gmt_modified: None,
            ..user
        },
    };
    app.user_manager.send(msg).await.ok();
    Ok(HttpResponse::Ok().json(ApiResult::success(Some(true))))
}

pub async fn update_user(
    app: Data<Arc<AppShareData>>,
    web::Json(user_param): web::Json<UpdateUserInfoParam>,
) -> actix_web::Result<impl Responder> {
    let user: UserDto = user_param.into();
    let msg = UserManagerReq::UpdateUser {
        user: UserDto {
            username: user.username,
            gmt_create: None,
            ..user
        },
    };
    app.user_manager.send(msg).await.ok();
    Ok(HttpResponse::Ok().json(ApiResult::success(Some(true))))
}

pub async fn remove_user(
    app: Data<Arc<AppShareData>>,
    web::Json(user): web::Json<UpdateUserInfoParam>,
) -> actix_web::Result<impl Responder> {
    let msg = UserManagerReq::Remove {
        username: user.username,
    };
    app.user_manager.send(msg).await.ok();
    Ok(HttpResponse::Ok().json(ApiResult::success(Some(true))))
}
