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

#[derive(Clone, PartialEq, prost_derive::Message, Deserialize, Serialize)]
pub struct NamespaceDO {
    #[prost(string, optional, tag = "1")]
    pub namespace_id: Option<String>,
    #[prost(string, optional, tag = "2")]
    pub namespace_name: Option<String>,
    #[prost(string, optional, tag = "3")]
    pub r#type: Option<String>,
}

impl NamespaceDO {
    pub fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        use prost::Message;
        let mut v = Vec::new();
        self.encode(&mut v)?;
        Ok(v)
    }

    pub fn from_bytes(data: &[u8]) -> anyhow::Result<Self> {
        use prost::Message;
        let s = Self::decode(data)?;
        Ok(s)
    }
}

impl From<NamespaceDO> for Namespace {
    fn from(value: NamespaceDO) -> Self {
        Self {
            namespace_id: Arc::new(value.namespace_id.unwrap_or_default()),
            namespace_name: value.namespace_name.unwrap_or_default(),
            r#type: value.r#type.unwrap_or("2".to_string()),
        }
    }
}

impl From<Namespace> for NamespaceDO {
    fn from(value: Namespace) -> Self {
        Self {
            namespace_id: Some(value.namespace_id.as_str().to_string()),
            namespace_name: Some(value.namespace_name),
            r#type: Some(value.r#type),
        }
    }
}

///
/// raft持久化后，向NamespaceActor发起的变更请求
///
#[derive(Message, Clone, Debug, Serialize, Deserialize)]
#[rtype(result = "anyhow::Result<NamespaceRaftResult>")]
pub enum NamespaceRaftReq {
    Update(NamespaceParam),
    Set(NamespaceParam),
    Delete { id: Arc<String> },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NamespaceRaftResult {
    None,
}

#[derive(Message, Clone, Debug, Serialize, Deserialize)]
#[rtype(result = "anyhow::Result<NamespaceQueryResult>")]
pub enum NamespaceQueryReq {
    List,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NamespaceQueryResult {
    List(Vec<Arc<Namespace>>),
    None,
}
