use openraft::{SnapshotMeta, BasicNode};
use prost::Message;
use serde::{Deserialize, Serialize};
use crate::raft::store::Request;

use super::NodeId;

#[derive(Clone, PartialEq, Message, Deserialize, Serialize)]
pub struct SnapshotDataInfo {
    #[prost(message,repeated, tag = "1")]
    pub items: Vec<SnapshotItem>,
    #[prost(string, tag = "2")]
    pub snapshot_meta_json: String,
}

impl SnapshotDataInfo {
    pub(crate) fn build_snapshot(&self) -> anyhow::Result<SnapshotMeta<NodeId, BasicNode>> {
        let snapshot = serde_json::from_str(&self.snapshot_meta_json)?;
        Ok(snapshot)
    }

    pub fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let mut data_bytes :Vec<u8>= Vec::new();
        self.encode(&mut data_bytes)?;
        Ok(data_bytes)
    }

    pub fn from_bytes(buf:&[u8]) -> anyhow::Result<Self> {
        let s = SnapshotDataInfo::decode(buf)?;
        Ok(s)
    }
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
pub struct RaftPayload {
    #[prost(string, tag = "1")]
    pub r#type: String,
    #[prost(bytes = "vec", tag = "2")]
    pub body: Vec<u8>,
}

impl From<RaftPayload> for Request {
    fn from(value: RaftPayload) -> Self {
        todo!()
    }
}
