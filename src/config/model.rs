use std::sync::Arc;
use actix::prelude::*;
use crate::config::core::ConfigKey;

#[derive(Message)]
#[rtype(result = "anyhow::Result<ConfigRaftResult>")]
pub enum ConfigRaftCmd {
    LoadSnapshot,
    ConfigAdd {
        key: String, value: Arc<String> ,history_id: u64,history_table_id:Option<u64>
    },
    ConfigRemove {
        key: String,
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

