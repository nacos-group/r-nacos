#![allow(unused_imports)]

use std::sync::Arc;

use crate::config::ConfigUtils;
use crate::grpc::HandlerResult;
use crate::{
    common::appdata::AppShareData,
    config::core::{ConfigActor, ConfigCmd, ConfigKey, ConfigResult, ListenerItem},
    grpc::{
        api_model::{
            BaseResponse, ConfigBatchListenRequest, ConfigChangeBatchListenResponse, ConfigContext,
            ConfigPublishRequest, ConfigQueryRequest, ConfigQueryResponse, ERROR_CODE,
            SUCCESS_CODE,
        },
        nacos_proto::Payload,
        PayloadHandler, PayloadUtils,
    },
};
use actix::prelude::Addr;
use async_trait::async_trait;

pub struct ConfigChangeBatchListenRequestHandler {
    app_data: Arc<AppShareData>,
}

impl ConfigChangeBatchListenRequestHandler {
    pub fn new(app_data: Arc<AppShareData>) -> Self {
        Self { app_data }
    }
}

#[async_trait]
impl PayloadHandler for ConfigChangeBatchListenRequestHandler {
    async fn handle(
        &self,
        request_payload: crate::grpc::nacos_proto::Payload,
        request_meta: crate::grpc::RequestMeta,
    ) -> anyhow::Result<HandlerResult> {
        let body_vec = request_payload.body.unwrap_or_default().value;
        let request: ConfigBatchListenRequest = serde_json::from_slice(&body_vec)?;
        let mut listener_items = vec![];
        for item in request.config_listen_contexts {
            let key = ConfigKey::new(
                &item.data_id,
                &item.group,
                &ConfigUtils::default_tenant(item.tenant),
            );
            listener_items.push(ListenerItem::new(key, item.md5));
        }
        let cmd = if request.listen {
            ConfigCmd::Subscribe(listener_items, request_meta.connection_id)
        } else {
            ConfigCmd::RemoveSubscribe(listener_items, request_meta.connection_id)
        };
        let mut response = ConfigChangeBatchListenResponse {
            request_id: request.request_id,
            message: Some("".to_string()),
            ..Default::default()
        };
        match self.app_data.config_addr.send(cmd).await {
            Ok(res) => {
                let r: ConfigResult = res.unwrap();
                match r {
                    ConfigResult::ChangeKey(keys) => {
                        response.result_code = SUCCESS_CODE;
                        for key in keys {
                            let obj = ConfigContext {
                                data_id: key.data_id,
                                group: key.group,
                                tenant: key.tenant,
                            };
                            response.changed_configs.push(obj);
                        }
                    }
                    _ => {
                        response.result_code = SUCCESS_CODE;
                    }
                }
                Ok(HandlerResult::success(PayloadUtils::build_payload(
                    "ConfigChangeBatchListenResponse",
                    serde_json::to_string(&response)?,
                )))
            }
            Err(err) => {
                response.result_code = ERROR_CODE;
                response.error_code = ERROR_CODE;
                response.message = Some(err.to_string());
                Ok(HandlerResult::success(PayloadUtils::build_payload(
                    "ErrorResponse",
                    serde_json::to_string(&response)?,
                )))
            }
        }
    }
}
