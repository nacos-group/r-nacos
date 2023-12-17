use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::user::{model::UserDto, permission::UserRoleHelper};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UserInfo {
    pub username: Option<Arc<String>>,
    pub nickname: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UserPermissions {
    pub resources: Vec<&'static str>,
    pub from: &'static str,
    pub version: &'static str,
    pub username: Option<Arc<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserInfoParam {
    pub username: Arc<String>,
    pub nickname: Option<String>,
    pub password: Option<String>,
    pub enable: Option<bool>,
    pub roles: Option<String>,
}

impl UpdateUserInfoParam {
    pub fn get_role_vec(&self) -> Option<Vec<Arc<String>>> {
        if let Some(roles) = self.roles.as_ref() {
            if roles.is_empty() {
                return None;
            }
            Some(
                roles
                    .split(',')
                    .map(|e| UserRoleHelper::get_role(e))
                    .collect(),
            )
        } else {
            None
        }
    }
}

impl From<UpdateUserInfoParam> for UserDto {
    fn from(value: UpdateUserInfoParam) -> Self {
        let roles = value.get_role_vec();
        Self {
            username: value.username,
            nickname: value.nickname,
            password: value.password,
            enable: value.enable,
            roles: roles,
            ..Default::default()
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserPageParams {
    pub like_username: Option<String>,
    pub is_rev: Option<bool>,
    pub page_no: Option<usize>,
    pub page_size: Option<usize>,
}

impl UserPageParams {
    pub fn get_limit_info(&self) -> (usize, usize) {
        let limit = self.page_size.unwrap_or(0xffff_ffff);
        let offset = (self.page_no.unwrap_or(1) - 1) * limit;
        (limit, offset)
    }
}
