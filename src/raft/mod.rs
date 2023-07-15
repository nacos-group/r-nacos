use std::sync::Arc;
use openraft::Raft;
use crate::raft::network::NetworkFactory;
use crate::raft::store::store::Store;
use crate::raft::store::TypeConfig;

pub mod network;
pub mod store;
pub type NacosRaft = Raft<TypeConfig, NetworkFactory, Arc<Store>>;