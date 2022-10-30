#![allow(unused_imports)]

use crate::{grpc::{PayloadHandler, api_model::{ConfigPublishRequest, BaseResponse, ConfigRemoveRequest}, nacos_proto::Payload, PayloadUtils}, config::config::{ConfigActor, ConfigCmd, ConfigKey, ConfigResult}};
use actix::prelude::Addr;
use async_trait::async_trait;

pub struct ConfigRemoveRequestHandler{
    config_addr: Addr<ConfigActor>,
}

impl ConfigRemoveRequestHandler {
    pub fn new(config_addr: Addr<ConfigActor>) -> Self {
        Self { config_addr }
    }
}

#[async_trait]
impl PayloadHandler for ConfigRemoveRequestHandler {
    async fn handle(&self, request_payload: crate::grpc::nacos_proto::Payload,_request_meta:crate::grpc::RequestMeta) -> anyhow::Result<Payload> {
        let body_vec = request_payload.body.unwrap_or_default().value;
        let request:ConfigRemoveRequest = serde_json::from_slice(&body_vec)?;
        let cmd = ConfigCmd::DELETE(ConfigKey::new(&request.data_id,&request.group,&request.tenant));
        let response = match self.config_addr.send(cmd).await{
            Ok(_res) => {
                //let res:ConfigResult = res.unwrap();
                BaseResponse::build_success_response()
            },
            Err(err) => {
                BaseResponse::build_error_response(500u16,err.to_string())
            }
        };
        Ok(PayloadUtils::build_payload("ConfigRemoveResponse", serde_json::to_string(&response)?))
    }
}
