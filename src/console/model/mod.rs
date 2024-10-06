pub mod cluster_model;
pub mod config_model;
pub mod login_model;
pub mod metrics_model;
pub mod naming_model;
pub mod paginate;
pub mod raft_model;
pub mod user_model;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NamespaceInfo {
    pub namespace_id: Option<String>,
    pub namespace_name: Option<String>,
    pub r#type: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConsoleResult<T>
where
    T: Sized + Serialize + Clone + Default,
{
    pub code: i64,
    pub message: Option<String>,
    pub data: Option<T>,
}

#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PageResult<T>
where
    T: Sized + Serialize + Clone + Default,
{
    pub count: u64,
    pub list: Vec<T>,
}

impl<T> ConsoleResult<T>
where
    T: Sized + Serialize + Clone + Default,
{
    pub fn success(data: T) -> Self {
        Self {
            code: 200,
            message: None,
            data: Some(data),
        }
    }
    pub fn error(message: String) -> Self {
        Self {
            code: 500,
            message: Some(message),
            data: None,
        }
    }
}
