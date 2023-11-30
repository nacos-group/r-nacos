use std::sync::Arc;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct LoginParam {
    pub username: Arc<String>,
    pub password: String,
    pub captcha: String,
}
