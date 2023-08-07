use async_raft::raft::{
    AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest, InstallSnapshotResponse,
    VoteRequest, VoteResponse,
};
use async_raft::{NodeId, RaftNetwork};
use async_trait::async_trait;
use std::sync::Arc;

use crate::grpc::nacos_proto::Payload;
use crate::grpc::PayloadUtils;
use crate::raft::store::core::RaftStore;
use crate::raft::store::ClientRequest;

use super::factory::RaftClusterRequestSender;

pub struct RaftRouter {
    store: Arc<RaftStore>, //get target addr
    cluster_sender: Arc<RaftClusterRequestSender>,
}

impl RaftRouter {
    pub fn new(store: Arc<RaftStore>, cluster_sender: Arc<RaftClusterRequestSender>) -> Self {
        Self {
            store,
            cluster_sender,
        }
    }

    async fn send_request(&self, target: u64, payload: Payload) -> anyhow::Result<Payload> {
        let addr = self.store.get_target_addr(target).await?;
        self.cluster_sender.send_request(addr, payload).await
    }
}

#[async_trait]
impl RaftNetwork<ClientRequest> for RaftRouter {
    async fn append_entries(
        &self,
        target: NodeId,
        req: AppendEntriesRequest<ClientRequest>,
    ) -> anyhow::Result<AppendEntriesResponse> {
        let request = serde_json::to_string(&req).unwrap_or_default();
        let payload = PayloadUtils::build_payload("RaftAppendRequest", request);
        let resp_payload = self.send_request(target, payload).await?;
        let body_vec = resp_payload.body.unwrap_or_default().value;
        let res: AppendEntriesResponse = serde_json::from_slice(&body_vec)?;
        Ok(res)
    }

    async fn install_snapshot(
        &self,
        target: NodeId,
        req: InstallSnapshotRequest,
    ) -> anyhow::Result<InstallSnapshotResponse> {
        let request = serde_json::to_string(&req).unwrap_or_default();
        let payload = PayloadUtils::build_payload("RaftSnapshotRequest", request);
        let resp_payload = self.send_request(target, payload).await?;
        let body_vec = resp_payload.body.unwrap_or_default().value;
        let res: InstallSnapshotResponse = serde_json::from_slice(&body_vec)?;
        Ok(res)
    }

    async fn vote(&self, target: NodeId, req: VoteRequest) -> anyhow::Result<VoteResponse> {
        let request = serde_json::to_string(&req).unwrap_or_default();
        let payload = PayloadUtils::build_payload("RaftVoteRequest", request);
        let resp_payload = self.send_request(target, payload).await?;
        let body_vec = resp_payload.body.unwrap_or_default().value;
        let res: VoteResponse = serde_json::from_slice(&body_vec)?;
        Ok(res)
    }
}

/*
pub struct HttpRaftRouter {
    //raft: MRaft,
    store: Arc<AStore>, //get target addr
    client: reqwest::Client,
}

impl HttpRaftRouter {

    pub fn new(store: Arc<AStore>)  -> Self {
        let client = reqwest::Client::new();
        Self {
            store,
            client,
        }
    }
}


#[async_trait]
impl RaftNetwork<ClientRequest> for HttpRaftRouter {
    async fn append_entries(&self, target: NodeId, req: AppendEntriesRequest<ClientRequest>) -> anyhow::Result<AppendEntriesResponse> {
        //send rpc request
        let addr = self.store.get_target_addr(target).await?;
        let url = format!("http://{}/nacos/v1/raft/append", &addr);
        let resp = self.client
            .post(url)
            .json(&req)
            .send()
            .await?;
        let res = resp.json().await?;
        Ok(res)
    }

    async fn install_snapshot(&self, target: NodeId, req: InstallSnapshotRequest) -> anyhow::Result<InstallSnapshotResponse> {
        let addr = self.store.get_target_addr(target).await?;
        let url = format!("http://{}/nacos/v1/raft/snapshot", &addr);
        let resp = self.client
            .post(url)
            .json(&req)
            .send()
            .await?;
        let res = resp.json().await?;
        Ok(res)
    }

    async fn vote(&self, target: NodeId, req: VoteRequest) -> anyhow::Result<VoteResponse> {
        let addr = self.store.get_target_addr(target).await?;
        let url = format!("http://{}/nacos/v1/raft/vote", &addr);
        let resp = self.client
            .post(url)
            .json(&req)
            .send()
            .await?;
        let res = resp.json().await?;
        Ok(res)
    }
}
 */
