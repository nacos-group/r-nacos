use crate::config::core::{ConfigHistoryInfoDto, ConfigKey};
use actix::prelude::*;
use std::sync::Arc;

#[derive(Message)]
#[rtype(result = "anyhow::Result<ConfigRaftResult>")]
pub enum ConfigRaftCmd {
    ConfigAdd {
        key: String,
        value: Arc<String>,
        history_id: u64,
        history_table_id: Option<u64>,
    },
    ConfigRemove {
        key: String,
    },
    ApplySnaphot,
}

pub enum ConfigRaftResult {
    Snapshot {
        data: Vec<(ConfigKey, Arc<String>)>,
        history_table_id: u64,
    },
    None,
}

pub struct HistoryItem{
    pub id: u64,
    pub content: Arc<String>,
    pub modified_time: i64,
}

impl HistoryItem {
    pub(crate) fn to_dto(&self,key:&ConfigKey) -> ConfigHistoryInfoDto {
        ConfigHistoryInfoDto{
            id: Some(self.id as i64),
            tenant: Some(key.tenant.to_string()),
            group: Some(key.group.to_string()),
            data_id: Some(key.data_id.to_string()),
            content: Some(self.content.to_string()),
            modified_time: Some(self.modified_time),
        }
    }
}