
pub mod serviceapi;

pub mod httpnetwork;

//use std::collections::HashMap;
use async_trait::async_trait;
use openraft::{AnyError, BasicNode, RaftNetwork, RaftNetworkFactory};
use openraft::error::{InstallSnapshotError, RaftError, RPCError};
use openraft::raft::{AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest, InstallSnapshotResponse, VoteRequest, VoteResponse};
use openraft::error::NetworkError;
use openraft::error::RemoteError;
use serde::de::DeserializeOwned;
use tonic::transport::Channel;
use crate::grpc::nacos_proto::Payload;
use crate::grpc::nacos_proto::request_client::RequestClient;
use crate::grpc::PayloadUtils;
use crate::raft::store::{NodeId, TypeConfig};

#[derive(Default, Debug, Clone)]
pub struct GrpcNetworkFactory {
    //connect_map: HashMap<NodeId,RaftNetworkConnect>
}

#[derive(Debug, Clone)]
pub struct GrpcRaftNetworkConnect {
    pub(crate) channel: Channel,
    pub(crate) target: NodeId,
    pub(crate) addr: String,
    //pub(crate) target_node: BasicNode,
}

impl GrpcRaftNetworkConnect {
    pub(crate) fn new(target: NodeId, target_node: BasicNode) -> Self {
        let addr = format!("http://{}", &target_node.addr);
        let channel = Channel::from_shared(addr.to_owned()).unwrap().connect_lazy().unwrap();
        Self {
            channel,
            target,
            addr,
            //target_node
        }
    }

    fn renew_conn(&mut self) {
        self.channel = Channel::from_shared(self.addr.to_owned()).unwrap().connect_lazy().unwrap();
    }

    async fn send_rpc<Resp, Err>(
        &mut self,
        payload: Payload,
    ) -> Result<Resp, RPCError<NodeId, BasicNode, Err>>
        where
            Err: std::error::Error + DeserializeOwned,
            Resp: DeserializeOwned,
    {
        let mut try_times=2;
        loop {
            match self.do_send_rpc(payload.clone()).await {
                Ok(v) => {
                    return Ok(v)
                },
                Err(err) => {
                    try_times -=1 ;
                    log::warn!("RaftNetworkConnect do_send_rpc err {}",&err.to_string());
                    if try_times==0 {
                        return Err(err)
                    }
                    self.renew_conn();
                },
            }
        }
    }

    async fn do_send_rpc<Resp, Err>(
        &mut self,
        payload: Payload,
    ) -> Result<Resp, RPCError<NodeId, BasicNode, Err>>
        where
            Err: std::error::Error + DeserializeOwned,
            Resp: DeserializeOwned,
    {
        let mut request_client = RequestClient::new(self.channel.clone());
        let target = self.target;
        let resp = request_client.request(payload).await.map_err(|e| RPCError::Network(NetworkError::new(&e)))?;
        let payload = resp.into_inner();
        //log::info!("rpc response:{}",PayloadUtils::get_payload_string(&payload));
        let body_vec = payload.body.unwrap_or_default().value;
        match serde_json::from_slice::<Result<Resp, Err>>(&body_vec) {
            Ok(v) => {
                return match v {
                    Ok(v) => {
                        Ok(v)
                    }
                    Err(err) => {
                        Err(RPCError::RemoteError(RemoteError::new(target, err)))
                    }
                };
            }
            Err(_e) => {
            }
        }
        Err(RPCError::Network(NetworkError::new(&AnyError::error("rpc err"))))
    }
}

async fn request_rpc(
    channel: Channel,
    payload: Payload,
) -> anyhow::Result<Payload> {
    let mut request_client = RequestClient::new(channel);
    let resp = request_client.request(payload).await?;
    let payload = resp.into_inner();
    Ok(payload)
}

async fn send_rpc<Resp, Err>(
    channel: Channel,
    payload: Payload,
    target: NodeId,
) -> Result<Resp, RPCError<NodeId, BasicNode, Err>>
    where
        Err: std::error::Error + DeserializeOwned,
        Resp: DeserializeOwned,
{
    let mut request_client = RequestClient::new(channel);
    let resp = request_client.request(payload).await.map_err(|e| RPCError::Network(NetworkError::new(&e)))?;
    let payload = resp.into_inner();
    //log::info!("rpc response:{}",PayloadUtils::get_payload_string(&payload));
    let body_vec = payload.body.unwrap_or_default().value;
    match serde_json::from_slice::<Result<Resp, Err>>(&body_vec) {
        Ok(v) => {
            return match v {
                Ok(v) => {
                    Ok(v)
                }
                Err(err) => {
                    Err(RPCError::RemoteError(RemoteError::new(target, err)))
                }
            };
        }
        Err(_e) => {
        }
    }
    Err(RPCError::Network(NetworkError::new(&AnyError::error("rpc err"))))
}

#[async_trait]
impl RaftNetworkFactory<TypeConfig> for GrpcNetworkFactory {
    type Network = GrpcRaftNetworkConnect;

    async fn new_client(&mut self, target: NodeId, node: &BasicNode) -> Self::Network {
        GrpcRaftNetworkConnect::new(target, node.to_owned())
        /* 
        if let Some(conn) = self.connect_map.get_mut(&target) {
            if conn.target_node.addr == node.addr {
                return conn.to_owned();
            }
        }
        let conn = RaftNetworkConnect::new(target,node.to_owned());
        self.connect_map.insert(target,conn.clone());
        conn
        */
    }
}

#[async_trait]
impl RaftNetwork<TypeConfig> for GrpcRaftNetworkConnect {
    async fn send_append_entries(&mut self, rpc: AppendEntriesRequest<TypeConfig>)
                                 -> Result<AppendEntriesResponse<NodeId>, RPCError<NodeId, BasicNode, RaftError<NodeId>>> {
        let request = serde_json::to_string(&rpc).unwrap_or_default();
        let payload = PayloadUtils::build_payload("RaftAppendRequest", request);
        //send_rpc(self.channel.clone(), payload, self.target.to_owned()).await
        self.send_rpc(payload).await
    }

    async fn send_install_snapshot(&mut self, rpc: InstallSnapshotRequest<TypeConfig>)
                                   -> Result<InstallSnapshotResponse<NodeId>, RPCError<NodeId, BasicNode, RaftError<NodeId, InstallSnapshotError>>> {
        let request = serde_json::to_string(&rpc).unwrap_or_default();
        let payload = PayloadUtils::build_payload("RaftSnapshotRequest", request);
        //send_rpc(self.channel.clone(), payload, self.target.to_owned()).await
        self.send_rpc(payload).await
    }

    async fn send_vote(&mut self, rpc: VoteRequest<NodeId>)
                       -> Result<VoteResponse<NodeId>, RPCError<NodeId, BasicNode, RaftError<NodeId>>> {
        let request = serde_json::to_string(&rpc).unwrap_or_default();
        let payload = PayloadUtils::build_payload("RaftVoteRequest", request);
        //send_rpc(self.channel.clone(), payload, self.target.to_owned()).await
        self.send_rpc(payload).await
    }
}