pub(crate) mod innerstore;
pub mod store;
pub mod model;

use std::error::Error;
use std::sync::Arc;

use openraft::AnyError;
use openraft::BasicNode;
use openraft::ErrorSubject;
use openraft::ErrorVerb;
use openraft::StorageError;
use openraft::StorageIOError;
use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Request {
    //Set { key: String, value: String },
    ConfigSet { key: String, value: Arc<String> ,history_id: u64,history_table_id:Option<u64>},
    ConfigRemove {key: String},
    //DbSet { table: String, key: Vec<u8>, value: Vec<u8> }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Response {
    pub value: Option<bool>,
}

pub type NodeId = u64;

openraft::declare_raft_types!(
    /// Declare the type configuration for example K/V store.
    pub TypeConfig: D = Request, R = Response, NodeId = NodeId, Node = BasicNode
);

pub(crate) fn sm_r_err<E: Error + 'static>(e: E) -> StorageError<NodeId> {
    StorageIOError::new(ErrorSubject::StateMachine, ErrorVerb::Read, AnyError::new(&e)).into()
}
pub(crate) fn sm_w_err<E: Error + 'static>(e: E) -> StorageError<NodeId> {
    StorageIOError::new(ErrorSubject::StateMachine, ErrorVerb::Write, AnyError::new(&e)).into()
}
pub(crate) fn s_r_err<E: Error + 'static>(e: E) -> StorageError<NodeId> {
    StorageIOError::new(ErrorSubject::Store, ErrorVerb::Read, AnyError::new(&e)).into()
}
pub(crate) fn s_w_err<E: Error + 'static>(e: E) -> StorageError<NodeId> {
    StorageIOError::new(ErrorSubject::Store, ErrorVerb::Write, AnyError::new(&e)).into()
}
pub(crate) fn v_r_err<E: Error + 'static>(e: E) -> StorageError<NodeId> {
    StorageIOError::new(ErrorSubject::Vote, ErrorVerb::Read, AnyError::new(&e)).into()
}
pub(crate) fn v_w_err<E: Error + 'static>(e: E) -> StorageError<NodeId> {
    StorageIOError::new(ErrorSubject::Vote, ErrorVerb::Write, AnyError::new(&e)).into()
}
pub(crate) fn l_r_err<E: Error + 'static>(e: E) -> StorageError<NodeId> {
    StorageIOError::new(ErrorSubject::Logs, ErrorVerb::Read, AnyError::new(&e)).into()
}
pub(crate) fn l_w_err<E: Error + 'static>(e: E) -> StorageError<NodeId> {
    StorageIOError::new(ErrorSubject::Logs, ErrorVerb::Write, AnyError::new(&e)).into()
}
pub(crate) fn m_r_err<E: Error + 'static>(e: E) -> StorageError<NodeId> {
    StorageIOError::new(ErrorSubject::Store, ErrorVerb::Read, AnyError::new(&e)).into()
}
pub(crate) fn m_w_err<E: Error + 'static>(e: E) -> StorageError<NodeId> {
    StorageIOError::new(ErrorSubject::Store, ErrorVerb::Write, AnyError::new(&e)).into()
}
pub(crate) fn t_err<E: Error + 'static>(e: E) -> StorageError<NodeId> {
    StorageIOError::new(ErrorSubject::Store, ErrorVerb::Write, AnyError::new(&e)).into()
}

pub(crate) fn ct_err<E: Error + 'static>(e: E) -> sled::transaction::ConflictableTransactionError<AnyError> {
    sled::transaction::ConflictableTransactionError::Abort(AnyError::new(&e))
}
