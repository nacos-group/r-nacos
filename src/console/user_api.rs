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
        constant::{APP_VERSION, EMPTY_STR},
        model::{ApiResult, PageResultOld, UserSession},
    },
    user::{model::UserDto, permission::UserRole, UserManagerReq, UserManagerResult},
};

use super::model::user_model::{UpdateUserInfoParam, UserInfo, UserPageParams, UserPermissions};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetPasswordParam {
    pub old_password: String,
    pub new_password: String,
}

pub async fn get_user_info(req: HttpRequest) -> actix_web::Result<impl Responder> {
    if let Some(session) = req.extensions().get::<Arc<UserSession>>() {
        let userinfo = UserInfo {
            username: Some(session.username.clone()),
            nickname: Some(session.nickname.clone()),
        };
        Ok(HttpResponse::Ok().json(ApiResult::success(Some(userinfo))))
    } else {
        Ok(HttpResponse::Ok().json(ApiResult::<()>::error(
            "NOT_FOUND_USER_SESSION".to_owned(),
            None,
        )))
    }
}

///
/// 获取用户权限资源列表
/// 这里把取不到UserSession当成旧控制台，后继可以考虑单独实现一个接口
pub async fn get_user_web_resources(req: HttpRequest) -> actix_web::Result<impl Responder> {
    if let Some(session) = req.extensions().get::<Arc<UserSession>>() {
        let resources = UserRole::get_web_resources_by_roles(
            session.roles.iter().map(|e| e.as_str()).collect(),
        );
        let data = UserPermissions {
            resources,
            from: EMPTY_STR,
            version: APP_VERSION,
            username: Some(session.username.clone()),
        };
        Ok(HttpResponse::Ok().json(ApiResult::success(Some(data))))
    } else {
        let resources = UserRole::OldConsole.get_web_resources();
        let data = UserPermissions {
            resources,
            from: "OLD_CONSOLE",
            version: APP_VERSION,
            username: None,
        };
        Ok(HttpResponse::Ok().json(ApiResult::success(Some(data))))
    }
}

pub async fn reset_password(
    req: HttpRequest,
    app: Data<Arc<AppShareData>>,
    web::Form(param): web::Form<ResetPasswordParam>,
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
        UserManagerResult::UserPageResult(size, list) => {
            Ok(HttpResponse::Ok().json(ApiResult::success(Some(PageResultOld { size, list }))))
        }
        _ => Ok(HttpResponse::Ok().json(ApiResult::<()>::error(
            "NOT_FOUND_USER_SESSION".to_owned(),
            Some("result type is error".to_owned()),
        ))),
    }
}

pub async fn add_user(
    app: Data<Arc<AppShareData>>,
    web::Form(user_param): web::Form<UpdateUserInfoParam>,
) -> actix_web::Result<impl Responder> {
    let user: UserDto = user_param.into();
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
    web::Form(user_param): web::Form<UpdateUserInfoParam>,
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
    web::Form(user): web::Form<UpdateUserInfoParam>,
) -> actix_web::Result<impl Responder> {
    let msg = UserManagerReq::Remove {
        username: user.username,
    };
    app.user_manager.send(msg).await.ok();
    Ok(HttpResponse::Ok().json(ApiResult::success(Some(true))))
}
