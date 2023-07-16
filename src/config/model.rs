use std::sync::Arc;
use actix::prelude::*;
use crate::config::core::ConfigKey;

#[derive(Message)]
#[rtype(result = "anyhow::Result<ConfigRaftResult>")]
pub enum ConfigRaftCmd {
    LoadSnapshot,
    ApplyLog{
        key: String, value: String ,history_id: u64,history_table_id:Option<u64>
    },
    ApplySnaphot{
        data: Vec<(String,Arc<String>)>,
        history_table_id: u64,
    }
}

pub enum ConfigRaftResult {
    Snapshot {
        data: Vec<(ConfigKey,Arc<String>)>,
        history_table_id: u64,
    },
    None,
}

