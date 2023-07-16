
use std::sync::Arc;

use async_trait::async_trait;
use crate::grpc::nacos_proto::Payload;
use crate::grpc::{PayloadHandler, PayloadUtils, RequestMeta};
use crate::raft::NacosRaft;
use crate::raft::store::{TypeConfig};

pub struct RaftAppendRequestHandler {
    raft : Arc<NacosRaft>,
}

impl RaftAppendRequestHandler{
    pub fn new(raft : Arc<NacosRaft>) -> Self {
        Self{
            raft
        }
    }
}

#[async_trait]
impl PayloadHandler for RaftAppendRequestHandler {
    async fn handle(&self, request_payload: Payload, _request_meta: RequestMeta) -> anyhow::Result<Payload> {
        let body_vec = request_payload.body.unwrap_or_default().value;
        let request: openraft::raft::AppendEntriesRequest<TypeConfig> = serde_json::from_slice(&body_vec)?;
        let res = self.raft.append_entries(request).await;
        let value = serde_json::to_string(&res)?;
        //log::info!("RaftAppendRequestHandler result:{}",&value);
        let payload = PayloadUtils::build_payload("RaftAppendResponse",value );
        Ok(payload)
    }
}