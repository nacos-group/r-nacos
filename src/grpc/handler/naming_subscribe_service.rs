#![allow(unused_imports)]

use std::sync::Arc;

use crate::grpc::HandlerResult;
use crate::{
    common::appdata::AppShareData,
    grpc::{
        api_model::{
            Instance as ApiInstance, ServiceInfo as ApiServiceInfo, SubscribeServiceRequest,
            SubscribeServiceResponse, ERROR_CODE, SUCCESS_CODE,
        },
        nacos_proto::Payload,
        PayloadHandler, PayloadUtils,
    },
    naming::{
        core::{NamingActor, NamingCmd, NamingResult},
        model::{Instance, ServiceInfo, ServiceKey},
        naming_subscriber::NamingListenerItem,
        NamingUtils,
    },
    now_millis_i64,
};
use actix::prelude::Addr;
use async_trait::async_trait;

use super::converter::ModelConverter;

pub struct SubscribeServiceRequestHandler {
    app_data: Arc<AppShareData>,
}

impl SubscribeServiceRequestHandler {
    pub fn new(app_data: Arc<AppShareData>) -> Self {
        Self { app_data }
    }

    fn convert_to_service_info(&self, info: ServiceInfo) -> ApiServiceInfo {
        ModelConverter::to_api_service_info(info)
    }

    fn build_subscribe_cmd(
        &self,
        subscribe: bool,
        service_key: ServiceKey,
        connection_id: Arc<String>,
    ) -> NamingCmd {
        let item = NamingListenerItem {
            service_key,
            clusters: None,
        };
        if subscribe {
            NamingCmd::Subscribe(vec![item], connection_id)
        } else {
            NamingCmd::RemoveSubscribe(vec![item], connection_id)
        }
    }
}

#[async_trait]
impl PayloadHandler for SubscribeServiceRequestHandler {
    async fn handle(
        &self,
        request_payload: crate::grpc::nacos_proto::Payload,
        request_meta: crate::grpc::RequestMeta,
    ) -> anyhow::Result<HandlerResult> {
        let body_vec = request_payload.body.unwrap_or_default().value;
        let request: SubscribeServiceRequest = serde_json::from_slice(&body_vec)?;
        let mut response = SubscribeServiceResponse {
            request_id: request.request_id,
            message: Some("".to_string()),
            ..Default::default()
        };
        let cluster = if let Some(v) = request.clusters.as_ref() {
            v.clone()
        } else {
            "".to_owned()
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
            &NamingUtils::default_group(request.group_name.unwrap_or_default()),
            &request.service_name.unwrap_or_default(),
        );
        let subscribe_cmd = self.build_subscribe_cmd(
            request.subscribe,
            key.clone(),
            request_meta.connection_id.clone(),
        );
        self.app_data.naming_addr.do_send(subscribe_cmd);
        let cmd = NamingCmd::QueryServiceInfo(key, cluster, true);
        match self.app_data.naming_addr.send(cmd).await {
            Ok(res) => {
                let result: NamingResult = res.unwrap();
                match result {
                    NamingResult::ServiceInfo(service_info) => {
                        let api_service_info = self.convert_to_service_info(service_info);
                        response.service_info = Some(api_service_info);
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
            "SubscribeServiceResponse",
            serde_json::to_string(&response)?,
        )))
    }
}
