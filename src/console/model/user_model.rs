use std::sync::Arc;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UserInfo {
    pub username: Option<Arc<String>>,
    pub nickname: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageParams {
    pub like_username: Option<String>,
    pub offset: Option<i64>,
    pub limit: Option<i64>,
    pub is_rev: Option<bool>,
}
