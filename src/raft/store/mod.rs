//pub mod core;
//pub mod innerstore;

use std::sync::Arc;

use super::db::table::TableManagerReq;
use crate::mcp::model::actor_model::{McpManagerRaftReq, McpManagerRaftResult};
use crate::namespace::model::NamespaceRaftReq;
use crate::sequence::model::{SequenceRaftReq, SequenceRaftResult};
use async_raft_ext::AppData;
use async_raft_ext::AppDataResponse;
use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

pub type NodeId = u64;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ClientRequest {
    NodeAddr {
        id: u64,
        addr: Arc<String>,
    },
    Members(Vec<u64>),
    ConfigSet {
        key: String,
        value: Arc<String>,
        config_type: Option<Arc<String>>,
        desc: Option<Arc<String>>,
        history_id: u64,
        history_table_id: Option<u64>,
        op_time: i64,
        op_user: Option<Arc<String>>,
    },
    ConfigFullValue {
        key: Vec<u8>,
        value: Vec<u8>,
        last_seq_id: Option<u64>,
    },
    ConfigRemove {
        key: String,
    },
    TableManagerReq(TableManagerReq),
    NamespaceReq(NamespaceRaftReq),
    SequenceReq {
        req: SequenceRaftReq,
    },
    McpReq {
        req: McpManagerRaftReq,
    },
}

impl AppData for ClientRequest {}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ClientResponse {
    Success,
    Fail,
    SequenceResp { resp: SequenceRaftResult },
    McpResp { resp: McpManagerRaftResult },
}

impl Default for ClientResponse {
    fn default() -> Self {
        Self::Success
    }
}

impl AppDataResponse for ClientResponse {}

#[derive(Clone, Debug, Error)]
pub enum ShutdownError {
    #[error("unsafe storage error")]
    UnsafeStorageError,
}

/*
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
 */
