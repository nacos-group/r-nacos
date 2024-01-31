pub mod core;
pub mod innerstore;

use std::collections::HashMap;
use std::sync::Arc;

use async_raft_ext::raft::MembershipConfig;
use async_raft_ext::AppData;
use async_raft_ext::AppDataResponse;
use prost::Message;
use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

use super::db::table::TableManagerReq;

pub type NodeId = u64;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ClientRequest {
    NodeAddr {
        id: u64,
        addr: Arc<String>,
    },
    ConfigSet {
        key: String,
        value: Arc<String>,
        history_id: u64,
        history_table_id: Option<u64>,
    },
    ConfigRemove {
        key: String,
    },
    TableManagerReq(TableManagerReq),
}

impl AppData for ClientRequest {}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ClientResponse {
    pub value: Option<Arc<String>>,
}

impl AppDataResponse for ClientResponse {}

#[derive(Clone, Debug, Error)]
pub enum ShutdownError {
    #[error("unsafe storage error")]
    UnsafeStorageError,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct StateMachine {
    pub last_applied_log: u64,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct RaftLog {
    pub index: u64,
    pub term: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct Membership {
    pub membership_config: Option<MembershipConfig>,
    pub node_addr: HashMap<u64, Arc<String>>,
}

#[derive(Clone, PartialEq, Deserialize, Serialize)]
pub struct SnapshotMeta {
    pub term: u64,
    /// The snapshot entry's index.
    pub index: u64,
    /// The latest membership configuration covered by the snapshot.
    pub membership: Membership,
}

#[derive(Clone, PartialEq, Message, Deserialize, Serialize)]
pub struct SnapshotItem {
    #[prost(uint32, tag = "1")]
    pub r#type: u32,
    #[prost(bytes = "vec", tag = "2")]
    pub key: Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub value: Vec<u8>,
}

#[derive(Clone, PartialEq, Message, Deserialize, Serialize)]
pub struct SnapshotDataInfo {
    #[prost(message, repeated, tag = "1")]
    pub items: Vec<SnapshotItem>,
    #[prost(string, tag = "2")]
    pub snapshot_meta_json: String,
    #[prost(map = "uint32, string", tag = "3")]
    pub table_map: ::std::collections::HashMap<u32, String>,
}

impl SnapshotDataInfo {
    pub(crate) fn build_snapshot_meta(&self) -> anyhow::Result<SnapshotMeta> {
        let snapshot = serde_json::from_str(&self.snapshot_meta_json)?;
        Ok(snapshot)
    }

    pub fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let mut data_bytes: Vec<u8> = Vec::new();
        self.encode(&mut data_bytes)?;
        Ok(data_bytes)
    }

    pub fn from_bytes(buf: &[u8]) -> anyhow::Result<Self> {
        let s = SnapshotDataInfo::decode(buf)?;
        Ok(s)
    }
}
