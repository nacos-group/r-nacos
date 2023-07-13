pub(crate) mod innerstore;
pub(crate) mod store;

use std::collections::BTreeMap;
use openraft::BasicNode;
use openraft::LogId;
use openraft::SnapshotMeta;
use openraft::StoredMembership;
use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Request {
    Set { key: String, value: String },
    //ConfigSet { key: String, value: String },
    //DbSet { table: String, key: Vec<u8>, value: Vec<u8> }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Response {
    pub value: Option<Vec<u8>>,
}

pub type NodeId = u64;

openraft::declare_raft_types!(
    /// Declare the type configuration for example K/V store.
    pub TypeConfig: D = Request, R = Response, NodeId = NodeId, Node = BasicNode
);

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct StateMachine {
    pub last_applied_log: Option<LogId<NodeId>>,

    pub last_membership: StoredMembership<NodeId, BasicNode>,

    /// Application data.
    pub data: BTreeMap<String, String>,
}

#[derive(Debug, Default, Clone)]
pub struct StoredSnapshot {
    pub meta: SnapshotMeta<NodeId, BasicNode>,

    /// The data of the state machine at the time of this snapshot.
    pub data: Vec<u8>,
}
