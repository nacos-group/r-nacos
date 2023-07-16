use prost::Message;
use serde::{Deserialize, Serialize};
use crate::raft::store::Request;

#[derive(Clone, PartialEq, Message, Deserialize, Serialize)]
pub struct SnapshotData {
    #[prost(int32, tag = "1")]
    pub r#type: i32,
}

#[derive(Clone, PartialEq, Message, Deserialize, Serialize)]
pub struct SnapshotItem {
    #[prost(int32, tag = "1")]
    pub r#type: i32,
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
