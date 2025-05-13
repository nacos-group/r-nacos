#![allow(unused_imports)]

use std::sync::Arc;

use actix::prelude::Addr;
use async_trait::async_trait;

use crate::grpc::HandlerResult;
use crate::{
    common::appdata::AppShareData,
    grpc::{
        api_model::{ServiceListRequest, ServiceListResponse, ERROR_CODE, SUCCESS_CODE},
        nacos_proto::Payload,
        PayloadHandler, PayloadUtils,
    },
    naming::{
        core::{NamingActor, NamingCmd, NamingResult},
        model::ServiceKey,
        NamingUtils,
    },
};

use super::converter::ModelConverter;

pub struct ServiceListRequestHandler {
    app_data: Arc<AppShareData>,
}

impl ServiceListRequestHandler {
    pub fn new(app_data: Arc<AppShareData>) -> Self {
        Self { app_data }
    }
}

#[async_trait]
impl PayloadHandler for ServiceListRequestHandler {
    async fn handle(
        &self,
        request_payload: crate::grpc::nacos_proto::Payload,
        _request_meta: crate::grpc::RequestMeta,
    ) -> anyhow::Result<HandlerResult> {
        let body_vec = request_payload.body.unwrap_or_default().value;
        let request: ServiceListRequest = serde_json::from_slice(&body_vec)?;
        let mut response = ServiceListResponse {
            request_id: request.request_id,
            message: Some("".to_string()),
            ..Default::default()
        };
        let namespace = NamingUtils::default_namespace(
            request
                .namespace
                .as_ref()
                .unwrap_or(&"".to_owned())
                .to_owned(),
        );
        let key = ServiceKey::new(
            &namespace,
            &request.group_name.unwrap_or_default(),
            &request.service_name.unwrap_or_default(),
        );
        let cmd =
            NamingCmd::QueryServicePage(key, request.page_size as usize, request.page_no as usize);
        match self.app_data.naming_addr.send(cmd).await {
            Ok(res) => {
                let result: NamingResult = res.unwrap();
                match result {
                    NamingResult::ServicePage((count, service_names)) => {
                        response.count = count;
                        response.service_names = Some(service_names);
                        response.result_code = SUCCESS_CODE;
                    }
                    _ => {
                        response.result_code = ERROR_CODE;
                        response.error_code = 500u16;
                        response.message = Some("naming handler result type is error".to_owned());
                    }
                };
            }
            Err(err) => {
                response.result_code = ERROR_CODE;
                response.error_code = 500u16;
                response.message = Some(err.to_string());
                return Ok(HandlerResult::success(PayloadUtils::build_payload(
                    "ErrorResponse",
                    serde_json::to_string(&response)?,
                )));
            }
        };
        Ok(HandlerResult::success(PayloadUtils::build_payload(
            "ServiceListResponse",
            serde_json::to_string(&response)?,
        )))
    }
}
