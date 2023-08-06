

pub mod innerstore;

pub mod store;

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Arc;

use async_raft::AppData;
use async_raft::AppDataResponse;
use async_raft::raft::MembershipConfig;
use prost::Message;
use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

pub type NodeId = u64;

/**
 * Here you will set the types of request that will interact with the raft nodes.
 * For example the `Set` will be used to write data (key and value) to the raft database.
 * The `AddNode` will append a new node to the current existing shared list of nodes.
 * You will want to add any request that can write data in all nodes here.
 */
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ClientRequest {
    //Set { key: Arc<String>, value: Arc<String> },
    NodeAddr{id:u64,addr:Arc<String>},
    ConfigSet { key: String, value: Arc<String> ,history_id: u64,history_table_id:Option<u64>},
    ConfigRemove {key: String},
}

impl AppData for ClientRequest {

}

/**
 * Here you will defined what type of answer you expect from reading the data of a node.
 * In this example it will return a optional value from a given key in
 * the `Request.Set`.
 *
 * TODO: Should we explain how to create multiple `AppDataResponse`?
 *
 */
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClientResponse {
    pub value: Option<Arc<String>>,
}

impl AppDataResponse for ClientResponse {

}

#[derive(Clone, Debug, Error)]
pub enum ShutdownError {
    #[error("unsafe storage error")]
    UnsafeStorageError,
}


/*
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StoredSnapshot {
    pub index: u64,
    /// The term of the last index covered by this snapshot.
    pub term: u64,
    /// The last memberhsip config included in this snapshot.
    pub membership: MembershipConfig,
    /// The data of the state machine at the time of this snapshot.
    pub data: Vec<u8>,
}
 */

/**
 * Here defines a state machine of the raft, this state represents a copy of the data
 * between each node. Note that we are using `serde` to serialize the `data`, which has
 * a implementation to be serialized. Note that for this test we set both the key and
 * value as String, but you could set any type of value that has the serialization impl.
 */
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct StateMachine {
    pub last_applied_log: u64,

    // Application data.
    //pub data: BTreeMap<Arc<String>, Arc<String>>,
}


#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct RaftLog {
    pub index: u64,
    pub term: u64,
}

#[derive(Serialize, Deserialize, Debug,Clone,PartialEq)]
pub struct Membership {
    pub membership_config: Option<MembershipConfig>,
    pub node_addr: HashMap<u64,Arc<String>>,
}

impl Default for Membership {
    fn default() -> Self {
        /*
        let membership_config = MembershipConfig{
            members: Default::default(),
            members_after_consensus: Default::default(),
        };
         */
        Self { 
            membership_config: None, 
            node_addr: Default::default() 
        }
    }
}

#[derive(Clone, PartialEq,Deserialize, Serialize)]
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
    #[prost(message,repeated, tag = "1")]
    pub items: Vec<SnapshotItem>,
    #[prost(string, tag = "2")]
    pub snapshot_meta_json: String,
}


impl SnapshotDataInfo {
    pub(crate) fn build_snapshot_meta(&self) -> anyhow::Result<SnapshotMeta> {
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