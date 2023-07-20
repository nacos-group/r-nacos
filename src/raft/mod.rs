use std::sync::Arc;
use openraft::Raft;
//use crate::raft::network::GrpcNetworkFactory;
use crate::raft::network::httpnetwork::HttpNetworkFactory;
use crate::raft::store::store::Store;
use crate::raft::store::TypeConfig;

pub mod network;
pub mod store;
//pub type NacosRaft = Raft<TypeConfig, GrpcNetworkFactory, Arc<Store>>;
pub type NacosRaft = Raft<TypeConfig, HttpNetworkFactory, Arc<Store>>;