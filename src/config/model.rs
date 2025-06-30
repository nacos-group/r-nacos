use crate::config::config_type::ConfigType;
use crate::config::core::{ConfigHistoryInfoDto, ConfigKey, ConfigValue};
use crate::utils::get_md5;
use actix::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Message)]
#[rtype(result = "anyhow::Result<ConfigRaftResult>")]
pub enum ConfigRaftCmd {
    ConfigAdd {
        key: String,
        value: Arc<String>,
        config_type: Option<Arc<String>>,
        desc: Option<Arc<String>>,
        history_id: u64,
        history_table_id: Option<u64>,
        op_time: i64,
        op_user: Option<Arc<String>>,
    },
    ConfigRemove {
        key: String,
    },
    SetFullValue {
        key: ConfigKey,
        value: ConfigValue,
        last_id: Option<u64>,
    },
}

#[derive(Debug)]
pub struct SetConfigParam {
    pub key: ConfigKey,
    pub value: Arc<String>,
    pub config_type: Option<Arc<String>>,
    pub desc: Option<Arc<String>>,
    pub history_id: u64,
    pub history_table_id: Option<u64>,
    pub op_time: i64,
    pub op_user: Option<Arc<String>>,
}

pub enum ConfigRaftResult {
    Snapshot {
        data: Vec<(ConfigKey, Arc<String>)>,
        history_table_id: u64,
    },
    None,
}

#[derive(Clone)]
pub struct HistoryItem {
    pub id: u64,
    pub content: Arc<String>,
    pub modified_time: i64,
    pub op_user: Option<Arc<String>>,
}

impl HistoryItem {
    pub(crate) fn to_dto(&self, key: &ConfigKey) -> ConfigHistoryInfoDto {
        ConfigHistoryInfoDto {
            id: Some(self.id as i64),
            tenant: Some(key.tenant.to_string()),
            group: Some(key.group.to_string()),
            data_id: Some(key.data_id.to_string()),
            content: Some(self.content.to_string()),
            modified_time: Some(self.modified_time),
            op_user: self.op_user.as_ref().map(|e| e.to_string()),
        }
    }
}

#[derive(Clone, PartialEq, prost_derive::Message, Deserialize, Serialize)]
pub struct ConfigHistoryItemDO {
    #[prost(uint64, optional, tag = "1")]
    pub id: Option<u64>,
    #[prost(string, optional, tag = "2")]
    pub content: Option<String>,
    #[prost(int64, optional, tag = "3")]
    pub last_time: Option<i64>,
    #[prost(string, optional, tag = "4")]
    pub op_user: Option<String>,
}

impl From<HistoryItem> for ConfigHistoryItemDO {
    fn from(value: HistoryItem) -> Self {
        Self {
            id: Some(value.id),
            content: Some(value.content.as_ref().to_string()),
            last_time: Some(value.modified_time),
            op_user: value.op_user.map(|e| e.as_ref().to_string()),
        }
    }
}

impl From<ConfigHistoryItemDO> for HistoryItem {
    fn from(value: ConfigHistoryItemDO) -> Self {
        Self {
            id: value.id.unwrap_or_default(),
            content: Arc::new(value.content.unwrap_or_default()),
            modified_time: value.last_time.unwrap_or_default(),
            op_user: value.op_user.map(Arc::new),
        }
    }
}
#[derive(Clone, PartialEq, prost_derive::Message, Deserialize, Serialize)]
pub struct ConfigValueDO {
    #[prost(string, optional, tag = "1")]
    pub content: Option<String>,
    #[prost(repeated, message, tag = "2")]
    pub histories: Vec<ConfigHistoryItemDO>,
    #[prost(string, optional, tag = "3")]
    pub config_type: Option<String>,
    #[prost(string, optional, tag = "4")]
    pub desc: Option<String>,
}

impl ConfigValueDO {
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

impl From<ConfigValue> for ConfigValueDO {
    fn from(value: ConfigValue) -> Self {
        Self {
            content: Some(value.content.as_ref().to_owned()),
            histories: value.histories.into_iter().map(|e| e.into()).collect(),
            config_type: value.config_type.map(|e| e.as_ref().to_owned()),
            desc: value.desc.map(|e| e.as_ref().to_owned()),
        }
    }
}

impl From<ConfigValueDO> for ConfigValue {
    fn from(value: ConfigValueDO) -> Self {
        let content = value.content.unwrap_or_default();
        let md5 = Arc::new(get_md5(&content));
        let last_modified =
            if let Some(Some(v)) = value.histories.iter().last().map(|e| e.last_time) {
                v
            } else {
                0
            };
        Self {
            content: Arc::new(content),
            md5,
            tmp: false,
            histories: value.histories.into_iter().map(|e| e.into()).collect(),
            config_type: value
                .config_type
                .map(|v| ConfigType::new_by_value(&v).get_value()),
            desc: value.desc.map(Arc::new),
            last_modified,
        }
    }
}
