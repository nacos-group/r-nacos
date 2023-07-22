
use std::sync::Arc;
use async_raft::{Config, NodeId, RaftNetwork};
use async_raft::raft::{AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest, InstallSnapshotResponse, VoteRequest, VoteResponse};
use async_trait::async_trait;

use crate::raft::asyncraft::store::ClientRequest;
use crate::raft::asyncraft::store::store::AStore;

pub struct RaftRouter {
    //raft: MRaft,
    store: Arc<AStore>, //get target addr
    client: reqwest::Client,
}

impl RaftRouter {

    pub fn new(store: Arc<AStore>)  -> Self {
        let client = reqwest::Client::new();
        Self {
            store,
            client,
        }
    }
}


#[async_trait]
impl RaftNetwork<ClientRequest> for RaftRouter {
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