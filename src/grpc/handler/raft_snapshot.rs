
use std::sync::Arc;

use async_trait::async_trait;
use crate::grpc::nacos_proto::Payload;
use crate::grpc::{PayloadHandler, PayloadUtils, RequestMeta};
use crate::raft::NacosRaft;
use crate::raft::store::TypeConfig;

pub struct RaftSnapshotRequestHandler {
    raft : Arc<NacosRaft>,
}

impl RaftSnapshotRequestHandler{
    pub fn new(raft : Arc<NacosRaft>) -> Self {
        Self{
            raft
        }
    }
}

#[async_trait]
impl PayloadHandler for RaftSnapshotRequestHandler {
    async fn handle(&self, request_payload: Payload, _request_meta: RequestMeta) -> anyhow::Result<Payload> {
        todo!()
        /*
        let body_vec = request_payload.body.unwrap_or_default().value;
        let request: openraft::raft::InstallSnapshotRequest<TypeConfig> = serde_json::from_slice(&body_vec)?;
        let res = self.raft.install_snapshot(request).await;
        let value = serde_json::to_string(&res)?;
        let payload = PayloadUtils::build_payload("RaftSnapshotResponse",value );
        Ok(payload)
         */
    }
}