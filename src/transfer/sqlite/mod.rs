use crate::common::constant::EMPTY_STR;
use crate::transfer::sqlite::dao::user::UserDO;
use crate::user::model::UserDo;
use std::collections::HashMap;

/// 支持r-nacos导出中间数据文件与sqlite数据库相互转化的模块
pub mod dao;

#[derive(Debug, Default)]
pub struct TableSeq {
    pub(crate) config_id: i64,
    pub(crate) config_history_id: i64,
    pub(crate) tenant_id: i64,
    pub(crate) user_id: i64,
}

impl TableSeq {
    pub fn next_config_id(&mut self) -> i64 {
        self.config_id += 1;
        self.config_id
    }

    pub fn next_config_history_id(&mut self) -> i64 {
        self.config_history_id += 1;
        self.config_history_id
    }

    pub fn next_tenant_id(&mut self) -> i64 {
        self.tenant_id += 1;
        self.tenant_id
    }
    pub fn next_user_id(&mut self) -> i64 {
        self.user_id += 1;
        self.user_id
    }
}

impl From<UserDo> for UserDO {
    fn from(value: UserDo) -> Self {
        Self {
            id: None,
            username: Some(value.username),
            nickname: Some(value.nickname),
            password_hash: value.password_hash,
            gmt_create: Some(value.gmt_create as i64),
            gmt_modified: Some(value.gmt_modified as i64),
            enabled: Some(value.enable.to_string()),
            roles: serde_json::to_string(&value.roles).ok(),
            extend_info: serde_json::to_string(&value.extend_info).ok(),
        }
    }
}

impl From<UserDO> for UserDo {
    fn from(v: UserDO) -> Self {
        let enable = if let Some(s) = &v.enabled {
            s.parse().unwrap_or_default()
        } else {
            false
        };
        let roles = if let Some(s) = &v.roles {
            serde_json::from_str(s).unwrap_or_default()
        } else {
            vec![]
        };
        let extend_info = if let Some(s) = &v.extend_info {
            serde_json::from_str(s).unwrap_or_default()
        } else {
            HashMap::default()
        };
        Self {
            username: v.username.unwrap_or_default(),
            password: EMPTY_STR.to_string(),
            nickname: v.nickname.unwrap_or_default(),
            gmt_create: v.gmt_create.unwrap_or_default() as u32,
            gmt_modified: v.gmt_modified.unwrap_or_default() as u32,
            enable,
            roles,
            extend_info,
            password_hash: v.password_hash,
            namespace_privilege_flags: None,
            namespace_white_list: Default::default(),
            namespace_black_list: Default::default(),
            source: None,
        }
    }
}
