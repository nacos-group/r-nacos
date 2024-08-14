use actix::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Namespace {
    pub namespace_id: Arc<String>,
    pub namespace_name: String,
    pub r#type: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct NamespaceParam {
    pub namespace_id: Arc<String>,
    pub namespace_name: Option<String>,
    pub r#type: Option<String>,
}

///
/// raft持久化后，向NamespaceActor发起的变更请求
///
#[derive(Message, Clone, Debug, Serialize, Deserialize)]
#[rtype(result = "anyhow::Result<NamespaceRaftResult>")]
pub enum NamespaceRaftReq {
    AddIfNotExist(NamespaceParam),
    Set(NamespaceParam),
    Delete { id: Arc<String> },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NamespaceRaftResult {
    None,
}
