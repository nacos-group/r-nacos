#![allow(unused_imports)]

use std::sync::Arc;

use crate::{
    config::core::{ConfigActor, ConfigCmd, ConfigKey, ConfigResult, ConfigAsyncCmd},
    grpc::{
        api_model::{BaseResponse, ConfigPublishRequest},
        nacos_proto::Payload,
        PayloadHandler, PayloadUtils,
    }, common::appdata::AppShareData, raft::cluster::model::SetConfigReq,
};
use actix::prelude::Addr;
use async_trait::async_trait;

pub struct ConfigPublishRequestHandler {
    app_data: Arc<AppShareData>, 
}

impl ConfigPublishRequestHandler {
    pub fn new(app_data: Arc<AppShareData>) -> Self {
        Self { app_data }
    }
}

#[async_trait]
impl PayloadHandler for ConfigPublishRequestHandler {
    async fn handle(
        &self,
        request_payload: crate::grpc::nacos_proto::Payload,
        _request_meta: crate::grpc::RequestMeta,
    ) -> anyhow::Result<Payload> {
        let body_vec = request_payload.body.unwrap_or_default().value;
        let request: ConfigPublishRequest = serde_json::from_slice(&body_vec)?;
        let req = SetConfigReq::new(
            ConfigKey::new(&request.data_id, &request.group, &request.tenant),
            request.content,
        );
        match self.app_data.config_route.set_config(req).await {
            Ok(_res) => {
                //let res:ConfigResult = res.unwrap();
                let mut response = BaseResponse::build_success_response();
                response.request_id = request.request_id;
                Ok(PayloadUtils::build_payload(
                    "ConfigPublishResponse",
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
