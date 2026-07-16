use crate::grpc::handler::{RAFT_APPEND_REQUEST, RAFT_SNAPSHOT_REQUEST, RAFT_VOTE_REQUEST};
use async_raft_ext::raft::{
    AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest, InstallSnapshotResponse,
    VoteRequest, VoteResponse,
};
use async_raft_ext::{NodeId, RaftNetwork};
use async_trait::async_trait;
use std::sync::Arc;

use crate::grpc::nacos_proto::Payload;
use crate::grpc::PayloadUtils;
use crate::raft::filestore::core::FileStore;
use crate::raft::store::ClientRequest;

use super::factory::RaftClusterRequestSender;

pub struct RaftRouter {
    store: Arc<FileStore>, //get target addr
    cluster_sender: Arc<RaftClusterRequestSender>,
}

impl RaftRouter {
    pub fn new(store: Arc<FileStore>, cluster_sender: Arc<RaftClusterRequestSender>) -> Self {
        Self {
            store,
            cluster_sender,
        }
    }

    async fn send_request(&self, target: u64, payload: Payload) -> anyhow::Result<Payload> {
        let addr = self.store.get_target_addr(target).await?;
        self.cluster_sender.send_request(addr, payload).await
    }

    /// 禁写闸门：本节点处于 close-write 状态时，拒绝一切出站 raft 网络请求。
    ///
    /// leader 的 heartbeat / 日志复制、follower 的投票请求都经此拦截，
    /// 使本节点不再向集群发送任何 raft 消息（含心跳），从而让其余节点
    /// 触发选举超时并选出新 leader，真正达成"等同关闭进程"的语义。
    fn ensure_not_close_write(&self) -> anyhow::Result<()> {
        if self.store.is_close_write() {
            return Err(anyhow::anyhow!(
                "raft is in close-write state, reject outbound raft network request"
            ));
        }
        Ok(())
    }
}

#[async_trait]
impl RaftNetwork<ClientRequest> for RaftRouter {
    async fn append_entries(
        &self,
        target: NodeId,
        req: AppendEntriesRequest<ClientRequest>,
    ) -> anyhow::Result<AppendEntriesResponse> {
        self.ensure_not_close_write()?;
        let request = serde_json::to_string(&req).unwrap_or_default();
        let payload = PayloadUtils::build_payload(RAFT_APPEND_REQUEST, request);
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
        self.ensure_not_close_write()?;
        let request = serde_json::to_string(&req).unwrap_or_default();
        let payload = PayloadUtils::build_payload(RAFT_SNAPSHOT_REQUEST, request);
        let resp_payload = self.send_request(target, payload).await?;
        let body_vec = resp_payload.body.unwrap_or_default().value;
        let res: InstallSnapshotResponse = serde_json::from_slice(&body_vec)?;
        Ok(res)
    }

    async fn vote(&self, target: NodeId, req: VoteRequest) -> anyhow::Result<VoteResponse> {
        self.ensure_not_close_write()?;
        let request = serde_json::to_string(&req).unwrap_or_default();
        let payload = PayloadUtils::build_payload(RAFT_VOTE_REQUEST, request);
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
