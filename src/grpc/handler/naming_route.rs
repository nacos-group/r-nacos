use std::sync::Arc;

use crate::{common::appdata::AppShareData, naming::cluster::{model::NamingRouteRequest, handle_naming_route}, grpc::{PayloadHandler, nacos_proto::Payload, RequestMeta, PayloadUtils}};
use async_trait::async_trait;


pub struct NamingRouteRequestHandler {
    app_data: Arc<AppShareData>,
}


impl NamingRouteRequestHandler {
    pub fn new(app_data: Arc<AppShareData>) -> Self {
        Self { app_data }
    }
}

#[async_trait]
impl PayloadHandler for NamingRouteRequestHandler {
    async fn handle(
        &self,
        request_payload: Payload,
        _request_meta: RequestMeta,
    ) -> anyhow::Result<Payload> {
        let body_vec = request_payload.body.unwrap_or_default().value;
        let request: NamingRouteRequest = serde_json::from_slice(&body_vec)?;
        let res = handle_naming_route(&self.app_data, request).await?;
        let value = serde_json::to_string(&res)?;
        let payload = PayloadUtils::build_payload("NamingRouteResponse", value);
        Ok(payload)
    }
}