pub mod client_version;
pub mod privilege;

use std::{collections::HashMap, sync::Arc};

pub use crate::common::model::client_version::ClientVersion;
use crate::common::model::privilege::PrivilegeGroup;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ApiResultOld<T>
where
    T: Sized + Default,
{
    pub data: Option<T>,
    pub success: bool,
    pub code: Option<String>,
    pub msg: Option<String>,
}

impl<T> ApiResultOld<T>
where
    T: Sized + Default,
{
    pub fn success(data: Option<T>) -> Self {
        Self {
            data,
            success: true,
            code: None,
            msg: None,
        }
    }

    pub fn error(code: String, msg: Option<String>) -> Self {
        Self {
            data: None,
            success: false,
            code: Some(code),
            msg,
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct PageResultOld<T> {
    pub size: usize,
    pub list: Vec<T>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ApiResult<T>
where
    T: Sized + Default,
{
    pub data: Option<T>,
    pub success: bool,
    pub code: Option<String>,
    pub message: Option<String>,
}

impl<T> crate::common::model::ApiResult<T>
where
    T: Sized + Default,
{
    pub fn success(data: Option<T>) -> Self {
        Self {
            data,
            success: true,
            code: None,
            message: None,
        }
    }

    pub fn error(code: String, message: Option<String>) -> Self {
        Self {
            data: None,
            success: false,
            code: Some(code),
            message,
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageResult<T> {
    pub total_count: usize,
    pub list: Vec<T>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct UserSession {
    pub username: Arc<String>,
    pub nickname: Option<String>,
    pub roles: Vec<Arc<String>>,
    pub namespace_privilege: Option<PrivilegeGroup<Arc<String>>>,
    pub extend_infos: HashMap<String, String>,
    /// 时间戳，单位秒
    pub refresh_time: u32,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct TokenSession {
    pub username: Arc<String>,
    pub roles: Vec<Arc<String>>,
    pub extend_infos: HashMap<String, String>,
}
