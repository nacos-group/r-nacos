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
