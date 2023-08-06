
use std::sync::Arc;

use async_trait::async_trait;
use crate::common::appdata::AppShareData;
use crate::grpc::nacos_proto::Payload;
use crate::grpc::{PayloadHandler, PayloadUtils, RequestMeta};
use crate::raft::cluster::handle_route;
use crate::raft::cluster::model::RouterRequest;

pub struct RaftRouteRequestHandler {
    app_data: Arc<AppShareData>, 
}

impl RaftRouteRequestHandler{
    pub fn new(app_data: Arc<AppShareData>) -> Self {
        Self { app_data }
    }
}

#[async_trait]
impl PayloadHandler for RaftRouteRequestHandler {
    async fn handle(&self, request_payload: Payload, _request_meta: RequestMeta) -> anyhow::Result<Payload> {
        let body_vec = request_payload.body.unwrap_or_default().value;
        let request: RouterRequest = serde_json::from_slice(&body_vec)?;
        let res=handle_route(&self.app_data,request).await?;
        let value = serde_json::to_string(&res)?;
        let payload = PayloadUtils::build_payload("RaftRouteResponse",value );
        Ok(payload)
    }
}