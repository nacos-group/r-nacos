use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use std::{collections::HashMap, convert::TryFrom, sync::Arc};

use crate::common::model::{TokenSession, UserSession};
use crate::common::pb::data_object::DirectCacheItemDo;

pub use crate::raft::cache::model::CacheKey;
pub use crate::raft::cache::model::CacheType;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CacheValue {
    String(Arc<String>),
    Number(i64),
    Map(Arc<HashMap<String, String>>),
    //后面UserSession换成定义好的对象
    UserSession(Arc<UserSession>),
    ApiTokenSession(Arc<TokenSession>),
}

impl Default for CacheValue {
    fn default() -> Self {
        Self::String(Default::default())
    }
}

impl CacheValue {
    pub fn try_to_number(&self) -> Option<i64> {
        match self {
            CacheValue::Number(v) => Some(*v),
            CacheValue::String(v) => v.parse::<i64>().ok(),
            _ => None,
        }
    }

    pub fn try_to_string(&self) -> Option<Arc<String>> {
        match self {
            CacheValue::String(v) => Some(v.clone()),
            CacheValue::Number(v) => Some(Arc::new(v.to_string())),
            _ => None,
        }
    }

    pub fn get_cache_type(&self) -> CacheType {
        match self {
            CacheValue::String(_) => CacheType::String,
            CacheValue::Number(_) => CacheType::String,
            CacheValue::Map(_) => CacheType::Map,
            CacheValue::UserSession(_) => CacheType::UserSession,
            CacheValue::ApiTokenSession(_) => CacheType::ApiTokenSession,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            CacheValue::String(v) => v.as_bytes().to_owned(),
            CacheValue::Number(v) => v.to_string().as_bytes().to_owned(),
            CacheValue::Map(m) => serde_json::to_vec(m).unwrap_or_default(),
            CacheValue::UserSession(v) => serde_json::to_vec(v).unwrap_or_default(),
            CacheValue::ApiTokenSession(v) => serde_json::to_vec(v).unwrap_or_default(),
        }
    }

    pub fn from_bytes(data: &[u8], cache_type: CacheType) -> anyhow::Result<Self> {
        match cache_type {
            CacheType::String => Ok(CacheValue::String(Arc::new(
                std::str::from_utf8(data)?.to_string(),
            ))),
            CacheType::Map => Ok(CacheValue::Map(Arc::new(serde_json::from_slice(&data)?))),
            CacheType::UserSession => Ok(CacheValue::UserSession(Arc::new(
                serde_json::from_slice(&data)?,
            ))),
            CacheType::ApiTokenSession => {
                Ok(CacheValue::ApiTokenSession(serde_json::from_slice(&data)?))
            }
        }
    }

    pub fn to_do<'b>(&self, key: &'b CacheKey) -> DirectCacheItemDo<'b> {
        DirectCacheItemDo {
            key: Cow::Borrowed(key.key.as_ref()),
            data: Cow::Owned(self.to_bytes()),
            timeout: 0,
            cache_type: self.get_cache_type().get_type_data() as u32,
        }
    }
}

impl<'a> TryFrom<DirectCacheItemDo<'a>> for CacheValue {
    type Error = anyhow::Error;
    fn try_from(value: DirectCacheItemDo<'a>) -> Result<Self, Self::Error> {
        let cache_type = CacheType::from_data(value.cache_type as u8)?;
        Self::from_bytes(value.data.as_ref(), cache_type)
    }
}

impl From<crate::raft::cache::model::CacheValue> for CacheValue {
    fn from(value: crate::raft::cache::model::CacheValue) -> Self {
        match value {
            crate::raft::cache::model::CacheValue::String(v) => CacheValue::String(v),
            crate::raft::cache::model::CacheValue::Map(v) => CacheValue::Map(v),
            crate::raft::cache::model::CacheValue::UserSession(v) => CacheValue::UserSession(v),
            crate::raft::cache::model::CacheValue::ApiTokenSession(v) => {
                CacheValue::ApiTokenSession(v)
            }
        }
    }
}
