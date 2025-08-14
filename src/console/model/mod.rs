pub mod cluster_model;
pub mod config_model;
pub mod login_model;
pub mod mcp_server_model;
pub mod mcp_tool_spec_model;
pub mod metrics_model;
pub mod naming_model;
pub mod raft_model;
pub mod user_model;

use crate::namespace::model::{Namespace, NamespaceFromFlags, NamespaceParam};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NamespaceInfo {
    pub namespace_id: Option<Arc<String>>,
    pub namespace_name: Option<String>,
    pub r#type: Option<String>,
}

impl From<Namespace> for NamespaceInfo {
    fn from(value: Namespace) -> Self {
        Self {
            namespace_id: Some(value.namespace_id),
            namespace_name: Some(value.namespace_name),
            r#type: Some(NamespaceFromFlags::get_api_type(value.flag)),
        }
    }
}

impl From<NamespaceInfo> for NamespaceParam {
    fn from(value: NamespaceInfo) -> Self {
        Self {
            namespace_id: value.namespace_id.unwrap_or_default(),
            namespace_name: value.namespace_name,
            r#type: value.r#type,
        }
    }
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
