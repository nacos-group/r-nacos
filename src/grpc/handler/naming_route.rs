use std::sync::Arc;

use crate::grpc::HandlerResult;
use crate::{
    common::appdata::AppShareData,
    grpc::{nacos_proto::Payload, PayloadHandler, PayloadUtils, RequestMeta},
    naming::cluster::{handle_naming_route, model::NamingRouteRequest},
};
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
    ) -> anyhow::Result<HandlerResult> {
        let body_vec = request_payload.body.unwrap_or_default().value;
        let request: NamingRouteRequest = serde_json::from_slice(&body_vec)?;
        let res = handle_naming_route(
            &self.app_data,
            request,
            request_payload
                .metadata
                .map(|e| e.headers)
                .unwrap_or_default(),
        )
        .await?;
        let value = serde_json::to_string(&res)?;
        let payload = PayloadUtils::build_payload("NamingRouteResponse", value);
        Ok(HandlerResult::success(payload))
    }
}
