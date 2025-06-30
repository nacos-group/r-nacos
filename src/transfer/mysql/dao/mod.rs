#![allow(clippy::field_reassign_with_default)]

use crate::common::constant::EMPTY_STR;
use crate::now_millis;
use crate::transfer::mysql::dao::user::UserDO;
use crate::user::model::UserDo;
use crate::user::permission::USER_ROLE_DEVELOPER;
use std::collections::HashMap;

pub mod config;
pub mod config_history;
pub mod tenant;
pub mod user;

impl From<UserDO> for UserDo {
    fn from(v: UserDO) -> Self {
        let enable = false;
        let roles = vec![USER_ROLE_DEVELOPER.as_str().to_owned()];
        let extend_info = HashMap::default();
        let nickname = v.username.clone().unwrap_or_default();
        let now = (now_millis() / 1000) as u32;
        Self {
            username: v.username.unwrap_or_default(),
            password: EMPTY_STR.to_string(),
            nickname,
            gmt_create: now,
            gmt_modified: now,
            enable,
            roles,
            extend_info,
            password_hash: v.password,
            namespace_privilege_flags: None,
            namespace_white_list: Default::default(),
            namespace_black_list: Default::default(),
            source: None,
        }
    }
}
