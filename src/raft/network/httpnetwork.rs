use std::sync::Arc;

use async_trait::async_trait;
use openraft::error::InstallSnapshotError;
use openraft::error::NetworkError;
use openraft::error::RPCError;
use openraft::error::RaftError;
use openraft::error::RemoteError;
use openraft::network::RaftNetwork;
use openraft::network::RaftNetworkFactory;
use openraft::raft::AppendEntriesRequest;
use openraft::raft::AppendEntriesResponse;
use openraft::raft::InstallSnapshotRequest;
use openraft::raft::InstallSnapshotResponse;
use openraft::raft::VoteRequest;
use openraft::raft::VoteResponse;
use openraft::BasicNode;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::raft::store::NodeId;
use crate::raft::store::TypeConfig;

#[derive(Clone)]
pub struct HttpNetworkFactory {
    pub(crate) client: Arc<reqwest::Client>,
}

impl HttpNetworkFactory {

    pub fn new() -> Self {
        let client = Arc::new(reqwest::Client::new());
        Self {
            client,
        }
    }

    pub async fn send_rpc<Req, Resp, Err>(
        &self,
        target: NodeId,
        target_node: &BasicNode,
        uri: &str,
        req: Req,
    ) -> Result<Resp, openraft::error::RPCError<NodeId, BasicNode, Err>>
    where
        Req: Serialize,
        Err: std::error::Error + DeserializeOwned,
        Resp: DeserializeOwned,
    {
        let addr = &target_node.addr;

        let url = format!("http://{}/nacos/v1/raft/{}", addr, uri);
        let resp = self.client
            .post(url)
            .json(&req)
            .send()
            .await
            .map_err(|e| openraft::error::RPCError::Network(NetworkError::new(&e)))?;
        let res: Result<Resp, Err> =
            resp.json().await.map_err(|e| openraft::error::RPCError::Network(NetworkError::new(&e)))?;

        res.map_err(|e| openraft::error::RPCError::RemoteError(RemoteError::new(target, e)))
    }
}

// NOTE: This could be implemented also on `Arc<ExampleNetwork>`, but since it's empty, implemented
// directly.
#[async_trait]
impl RaftNetworkFactory<TypeConfig> for HttpNetworkFactory {
    type Network = HttpNetworkConnection;

    async fn new_client(&mut self, target: NodeId, node: &BasicNode) -> Self::Network {
        HttpNetworkConnection {
            owner: self.clone(),
            target,
            target_node: node.clone(),
        }
    }
}

pub struct HttpNetworkConnection {
    owner: HttpNetworkFactory,
    target: NodeId,
    target_node: BasicNode,
}

#[async_trait]
impl RaftNetwork<TypeConfig> for HttpNetworkConnection {
    async fn send_append_entries(&mut self, req: AppendEntriesRequest<TypeConfig>)
                                 -> Result<AppendEntriesResponse<NodeId>, RPCError<NodeId, BasicNode, RaftError<NodeId>>> {
        self.owner.send_rpc(self.target, &self.target_node, "raft-append", req).await
    }

    async fn send_install_snapshot(&mut self, req: InstallSnapshotRequest<TypeConfig>)
                                   -> Result<InstallSnapshotResponse<NodeId>, RPCError<NodeId, BasicNode, RaftError<NodeId, InstallSnapshotError>>> {
        self.owner.send_rpc(self.target, &self.target_node, "raft-snapshot", req).await
    }

    async fn send_vote(&mut self, req: VoteRequest<NodeId>)
                       -> Result<VoteResponse<NodeId>, RPCError<NodeId, BasicNode, RaftError<NodeId>>> {
        self.owner.send_rpc(self.target, &self.target_node, "raft-vote", req).await
    }
}