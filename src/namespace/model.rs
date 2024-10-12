use actix::prelude::*;
use bitflags::bitflags;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/*
lazy_static::lazy_static! {
    pub static ref FROM_SYSTEM_ARC_VALUE: Arc<String> = Arc::new("0".to_string());
    pub static ref FROM_USER_ARC_VALUE: Arc<String> = Arc::new("2".to_string());
}
*/

pub(crate) const FROM_SYSTEM_VALUE: &'static str = "0";
pub(crate) const FROM_USER_VALUE: &'static str = "2";

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Namespace {
    pub namespace_id: Arc<String>,
    pub namespace_name: String,
    //pub r#type: String,
    pub flag: u32,
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
        let flag = if let Some(t) = &value.r#type {
            NamespaceFromFlags::from_db_type(t)
        } else {
            NamespaceFromFlags::USER.bits()
        };
        Self {
            namespace_id: Arc::new(value.namespace_id.unwrap_or_default()),
            namespace_name: value.namespace_name.unwrap_or_default(),
            flag,
        }
    }
}

impl From<Namespace> for NamespaceDO {
    fn from(value: Namespace) -> Self {
        let t = NamespaceFromFlags::get_db_type(value.flag);
        Self {
            namespace_id: Some(value.namespace_id.as_str().to_string()),
            namespace_name: Some(value.namespace_name),
            r#type: Some(t),
        }
    }
}

bitflags! {
    /// Represents a set of flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct NamespaceFromFlags: u32 {
        /// The value `SYSTEM`, at bit position `0`.
        const SYSTEM = 0b00000001;
        /// The value `USER`, at bit position `1`.
        const USER = 0b00000010;
        /// The value `CONFIG`, at bit position `2`.
        const CONFIG = 0b00000100;
        /// The value `NAMING`, at bit position `3`.
        const NAMING= 0b00001000;

        /// The combination of `A`, `B`, and `C`.
        const CONFIG_NAMING = Self::CONFIG.bits() | Self::NAMING.bits() ;
    }
}

impl NamespaceFromFlags {
    pub fn from_db_type(t: &str) -> u32 {
        if t == FROM_SYSTEM_VALUE {
            Self::SYSTEM.bits()
        } else {
            Self::USER.bits()
            //t.parse().unwrap_or(Self::USER.bits())
        }
    }

    pub fn get_db_type(v: u32) -> String {
        if v == Self::SYSTEM.bits() {
            FROM_SYSTEM_VALUE.to_string()
        } else {
            //v.to_string()
            FROM_USER_VALUE.to_string()
        }
    }
    pub fn get_api_type(v: u32) -> String {
        if v == Self::SYSTEM.bits() {
            FROM_SYSTEM_VALUE.to_string()
        } else {
            v.to_string()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WeakNamespaceFromType {
    Config,
    Naming,
}

impl WeakNamespaceFromType {
    pub fn get_flag(&self) -> u32 {
        match self {
            WeakNamespaceFromType::Config => NamespaceFromFlags::CONFIG.bits(),
            WeakNamespaceFromType::Naming => NamespaceFromFlags::NAMING.bits(),
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
