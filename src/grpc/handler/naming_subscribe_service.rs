#![allow(unused_imports)]

use crate::{grpc::{PayloadHandler, api_model::{SUCCESS_CODE, ERROR_CODE, Instance as ApiInstance, SubscribeServiceRequest, SubscribeServiceResponse}, nacos_proto::Payload, PayloadUtils}, naming::{core::{NamingActor, NamingCmd}, model::Instance, NamingUtils}, now_millis_i64};
use actix::prelude::Addr;
use async_trait::async_trait;

pub struct SubscribeServiceRequestHandler{
    naming_addr: Addr<NamingActor>,
}

impl SubscribeServiceRequestHandler {
    pub fn new(naming_addr: Addr<NamingActor>) -> Self {
        Self { naming_addr }
    }
}


#[async_trait]
impl PayloadHandler for SubscribeServiceRequestHandler {
    async fn handle(&self, request_payload: crate::grpc::nacos_proto::Payload,_request_meta:crate::grpc::RequestMeta) -> anyhow::Result<Payload> {
        let body_vec = request_payload.body.unwrap_or_default().value;
        let _request:SubscribeServiceRequest = serde_json::from_slice(&body_vec)?;
        let mut response = SubscribeServiceResponse::default();
        //TODO
        response.result_code=ERROR_CODE;
        response.error_code=302u16;
        response.message=Some("TODO...".to_owned());
        Ok(PayloadUtils::build_payload("SubscribeServiceResponse", serde_json::to_string(&response)?))
    }
}
