///目前的接口提供给测试验证使用，后续需要按需调整
use std::sync::Arc;

use actix_web::{
    web::{self, Data, Json},
    Responder,
};
use serde::{Deserialize, Serialize};

use crate::common::appdata::AppShareData;

use super::{
    model::{UserDo, UserDto},
    UserManagerReq,
};

#[derive(Debug, Deserialize, Serialize)]
pub struct UserVo {
    pub username: Arc<String>,
    pub password: Option<String>,
    pub nickname: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PageParams {
    like_username: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
    is_rev: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PageResult<T> {
    size: usize,
    list: Vec<T>,
}

impl From<UserDo> for UserVo {
    fn from(value: UserDo) -> Self {
        Self {
            username: Arc::new(value.username),
            password: Some(value.password),
            nickname: Some(value.nickname),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UserResult<T> {
    pub success: bool,
    pub msg: Option<String>,
    pub data: Option<T>,
}

pub async fn add_user(
    app: Data<Arc<AppShareData>>,
    web::Form(param): web::Form<UserVo>,
) -> actix_web::Result<impl Responder> {
    let msg = UserManagerReq::AddUser {
        user: UserDto {
            username: param.username,
            nickname: Some(param.nickname.unwrap()),
            password: Some(param.password.unwrap()),
            ..Default::default()
        },
        namespace_privilege_param: None,
    };
    app.user_manager.send(msg).await.ok();
    Ok("{\"ok\":1}")
}

pub async fn update_user(
    app: Data<Arc<AppShareData>>,
    web::Form(param): web::Form<UserVo>,
) -> actix_web::Result<impl Responder> {
    let msg = UserManagerReq::UpdateUser {
        user: UserDto {
            username: param.username,
            nickname: Some(param.nickname.unwrap()),
            password: Some(param.password.unwrap()),
            ..Default::default()
        },
        namespace_privilege_param: None,
    };
    app.user_manager.send(msg).await.ok();
    Ok("{\"ok\":1}")
}

pub async fn check_user(
    app: Data<Arc<AppShareData>>,
    web::Form(param): web::Form<UserVo>,
) -> actix_web::Result<impl Responder> {
    let msg = UserManagerReq::CheckUser {
        name: param.username,
        password: param.password.unwrap(),
    };
    match app.user_manager.send(msg).await.unwrap().unwrap() {
        super::UserManagerResult::CheckUserResult(v, _) => Ok(Json(UserResult {
            success: true,
            msg: None,
            data: Some(v),
        })),
        _ => Ok(Json(UserResult {
            success: false,
            msg: Some("result type is error".to_owned()),
            data: None,
        })),
    }
}

pub async fn get_user(
    app: Data<Arc<AppShareData>>,
    web::Query(param): web::Query<UserVo>,
) -> actix_web::Result<impl Responder> {
    let msg = UserManagerReq::Query {
        name: param.username,
    };
    match app.user_manager.send(msg).await.unwrap().unwrap() {
        super::UserManagerResult::QueryUser(user) => Ok(Json(UserResult::<UserDto> {
            success: true,
            msg: None,
            data: user,
        })),
        _ => Ok(Json(UserResult {
            success: false,
            msg: Some("result type is error".to_owned()),
            data: None,
        })),
    }
}

pub async fn get_user_page_list(
    app: Data<Arc<AppShareData>>,
    web::Query(param): web::Query<PageParams>,
) -> actix_web::Result<impl Responder> {
    let msg = UserManagerReq::QueryPageList {
        like_username: param.like_username,
        offset: param.offset,
        limit: param.limit,
        is_rev: param.is_rev.unwrap_or_default(),
    };
    match app.user_manager.send(msg).await.unwrap().unwrap() {
        super::UserManagerResult::UserPageResult(size, list) => Ok(Json(UserResult {
            success: true,
            msg: None,
            data: Some(PageResult { size, list }),
        })),
        _ => Ok(Json(UserResult {
            success: false,
            msg: Some("result type is error".to_owned()),
            data: None,
        })),
    }
}
