use std::sync::Arc;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoginParam {
    pub username: Arc<String>,
    pub password: String,
    pub captcha: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LoginToken {
    pub token: String,
}
