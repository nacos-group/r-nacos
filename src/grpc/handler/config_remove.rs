#![allow(unused_imports)]

use crate::{
    config::core::{ConfigActor, ConfigCmd, ConfigKey, ConfigResult, ConfigAsyncCmd},
    grpc::{
        api_model::{BaseResponse, ConfigPublishRequest, ConfigRemoveRequest},
        nacos_proto::Payload,
        PayloadHandler, PayloadUtils,
    },
};
use actix::prelude::Addr;
use async_trait::async_trait;

pub struct ConfigRemoveRequestHandler {
    config_addr: Addr<ConfigActor>,
}

impl ConfigRemoveRequestHandler {
    pub fn new(config_addr: Addr<ConfigActor>) -> Self {
        Self { config_addr }
    }
}

#[async_trait]
impl PayloadHandler for ConfigRemoveRequestHandler {
    async fn handle(
        &self,
        request_payload: crate::grpc::nacos_proto::Payload,
        _request_meta: crate::grpc::RequestMeta,
    ) -> anyhow::Result<Payload> {
        let body_vec = request_payload.body.unwrap_or_default().value;
        let request: ConfigRemoveRequest = serde_json::from_slice(&body_vec)?;
        let cmd = ConfigAsyncCmd::Delete(ConfigKey::new(
            &request.data_id,
            &request.group,
            &request.tenant,
        ));
        match self.config_addr.send(cmd).await {
            Ok(_res) => {
                let mut response = BaseResponse::build_success_response();
                response.request_id = request.request_id;
                Ok(PayloadUtils::build_payload(
                    "ConfigRemoveResponse",
                    serde_json::to_string(&response)?,
                ))
            }
            Err(err) => {
                let mut response = BaseResponse::build_error_response(500u16, err.to_string());
                response.request_id = request.request_id;
                Ok(PayloadUtils::build_payload(
                    "ErrorResponse",
                    serde_json::to_string(&response)?,
                ))
            }
        }
    }
}
