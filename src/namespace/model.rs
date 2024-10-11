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

pub(crate) const FROM_SYSTEM_VALUE: &'static str = "0";
pub(crate) const FROM_USER_VALUE: &'static str = "2";
pub(crate) const FROM_CONFIG_VALUE: &'static str = "3";
pub(crate) const FROM_NAMING_VALUE: &'static str = "4";
pub(crate) const FROM_CONFIG_OR_NAMING_VALUE: &'static str = "5";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WeakNamespaceFromType {
    Config,
    Naming,
}

impl WeakNamespaceFromType {
    pub fn get_type_value(&self) -> &str {
        match self {
            WeakNamespaceFromType::Config => FROM_CONFIG_VALUE,
            WeakNamespaceFromType::Naming => FROM_NAMING_VALUE,
        }
    }

    /// 获取合并弱引用类型后的值
    pub fn get_merge_value<'a>(&self, v: &'a str) -> &'a str {
        if v == FROM_SYSTEM_VALUE || v == FROM_USER_VALUE || self.get_type_value() == v {
            v
        } else {
            FROM_CONFIG_OR_NAMING_VALUE
        }
    }

    /// 获取分离弱引用类型后的值
    pub fn get_split_value<'a>(&self, v: &'a str) -> Option<&'a str> {
        if v == FROM_CONFIG_OR_NAMING_VALUE {
            //合并值需要拆开
            match self {
                WeakNamespaceFromType::Config => Some(FROM_NAMING_VALUE),
                WeakNamespaceFromType::Naming => Some(FROM_CONFIG_VALUE),
            }
        } else if v != self.get_type_value() {
            //非合并值，且与当前类型值不相等，直接返回原值
            Some(v)
        } else {
            //否则移除自身，返回空
            None
        }
    }

    /// 判断指定值是否已包含当前类型或已持久化,以确认是否要更新类型值;
    pub fn contained_by_value(&self, v: &str) -> bool {
        if v == FROM_SYSTEM_VALUE || v == FROM_USER_VALUE || v == FROM_CONFIG_OR_NAMING_VALUE {
            true
        } else {
            self.get_type_value() == v
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeakNamespaceParam {
    pub namespace_id: Arc<String>,
    pub from_type: WeakNamespaceFromType,
}

///
/// raft持久化后，向NamespaceActor发起的变更请求
///
#[derive(Message, Clone, Debug, Serialize, Deserialize)]
#[rtype(result = "anyhow::Result<NamespaceRaftResult>")]
pub enum NamespaceRaftReq {
    AddOnly(NamespaceParam),
    Update(NamespaceParam),
    Set(NamespaceParam),
    Delete {
        id: Arc<String>,
    },
    /// 从原配置中心旧的数据初始化
    InitFromOldValue(Arc<String>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NamespaceRaftResult {
    None,
}

#[derive(Message, Clone, Debug)]
#[rtype(result = "anyhow::Result<NamespaceActorResult>")]
pub enum NamespaceActorReq {
    SetWeak(WeakNamespaceParam),
    RemoveWeak(WeakNamespaceParam),
}

#[derive(Clone, Debug)]
pub enum NamespaceActorResult {
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
