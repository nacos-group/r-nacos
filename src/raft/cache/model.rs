use std::{sync::Arc, collections::HashMap};



#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum CacheType {
    String,
    Map,
    UserSession,
    //TokenSession,
}

impl Default for CacheType {
    fn default() -> Self {
        Self::String
    }
}

impl CacheType {
    pub fn get_type_data(&self) -> u8 {
        match self {
            CacheType::String => 1,
            CacheType::Map => 2,
            CacheType::UserSession => 10,
            //CacheType::TokenSession => 11,
        }
    }

    pub fn from_data(&self,v:u8) -> anyhow::Result<Self> {
        match v {
            1 => Ok(CacheType::String),
            2 => Ok(CacheType::Map),
            10 => Ok(CacheType::UserSession),
            //11 => Ok(CacheType::TokenSession),
            _ => Err(anyhow::anyhow!("unknown type from {}",&v)),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Hash, Default)]
pub struct CacheKey {
    pub cache_type: CacheType,
    pub key: Arc<String>,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum CacheValue {
    String(String),
    Map(HashMap<String,String>),
    //后面UserSession换成定义好的对象
    UserSession(HashMap<String,String>),
    //TokenSession(HashMap<String,String>),
}

impl Default for CacheValue {
    fn default() -> Self {
        Self::String(Default::default())
    }
}

impl CacheValue {
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            CacheValue::String(v) => v.as_bytes().to_owned(),
            CacheValue::Map(m) => serde_json::to_vec(m).unwrap(),
            CacheValue::UserSession(v) => serde_json::to_vec(v).unwrap(),
            //CacheValue::TokenSession(v) => serde_json::to_vec(v).unwrap(),
        }
    }

    pub fn from_bytes(data: Vec<u8>,t:CacheType) -> anyhow::Result<Self> {
        match t {
            CacheType::String => Ok(CacheValue::String(String::from_utf8(data)?)),
            CacheType::Map => Ok(CacheValue::Map(serde_json::from_slice(&data)?)),
            CacheType::UserSession => Ok(CacheValue::UserSession(serde_json::from_slice(&data)?)),
            //CacheType::TokenSession => Ok(CacheValue::TokenSession(serde_json::from_slice(&data)?)),
        }
    }
}