use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ApiResult<T>
where
    T: Sized + Default,
{
    pub data: Option<T>,
    pub success: bool,
    pub code: Option<String>,
    pub msg: Option<String>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct UserSession {
    pub name: String,
    pub roles: Vec<String>,
    pub extend_infos: HashMap<String, String>,
}
