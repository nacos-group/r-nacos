use std::sync::Arc;
use async_raft::Raft;

//use openraft::Raft;
//use crate::raft::network::GrpcNetworkFactory;
//use crate::raft::network::httpnetwork::HttpNetworkFactory;
//use crate::raft::store::store::Store;
//use crate::raft::store::TypeConfig;

use self::asyncraft::{store::{ClientRequest, ClientResponse, store::AStore}};
use self::asyncraft::network::network::RaftRouter;

pub mod network;
pub mod store;
pub mod asyncraft;
//pub type NacosRaft = Raft<TypeConfig, GrpcNetworkFactory, Arc<Store>>;
//pub type NacosRaft = Raft<TypeConfig, HttpNetworkFactory, Arc<Store>>;
pub type NacosRaft = Raft<ClientRequest, ClientResponse, RaftRouter, AStore>;
