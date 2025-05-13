#![allow(unused_imports)]

use std::sync::Arc;

use crate::common::model::client_version::ClientNameType;
use crate::config::config_type::ConfigType;
use crate::config::ConfigUtils;
use crate::grpc::api_model::NOT_FOUND;
use crate::grpc::HandlerResult;
use crate::{
    common::appdata::AppShareData,
    config::core::{ConfigActor, ConfigCmd, ConfigKey, ConfigResult},
    grpc::{
        api_model::{
            BaseResponse, ConfigPublishRequest, ConfigQueryRequest, ConfigQueryResponse,
            ERROR_CODE, SUCCESS_CODE,
        },
        nacos_proto::Payload,
        PayloadHandler, PayloadUtils,
    },
};
use actix::prelude::Addr;
use async_trait::async_trait;

pub struct ConfigQueryRequestHandler {
    app_data: Arc<AppShareData>,
}

impl ConfigQueryRequestHandler {
    pub fn new(app_data: Arc<AppShareData>) -> Self {
        Self { app_data }
    }
}

#[async_trait]
impl PayloadHandler for ConfigQueryRequestHandler {
    async fn handle(
        &self,
        request_payload: crate::grpc::nacos_proto::Payload,
        request_meta: crate::grpc::RequestMeta,
    ) -> anyhow::Result<HandlerResult> {
        let body_vec = request_payload.body.unwrap_or_default().value;
        let request: ConfigQueryRequest = serde_json::from_slice(&body_vec)?;
        let cmd = ConfigCmd::GET(ConfigKey::new(
            &request.data_id,
            &request.group,
            &ConfigUtils::default_tenant(request.tenant),
        ));
        let mut response = ConfigQueryResponse {
            request_id: request.request_id,
            ..Default::default()
        };
        if request_meta.client_version.client.is_python_sdk() {
            response.message = Some("".to_string());
            response.tag = Some("false".to_string());
            response.encrypted_data_key = Some("".to_string());
            response.beta = false;
        }
        match self.app_data.config_addr.send(cmd).await {
            Ok(res) => {
                //let res:ConfigResult = res.unwrap();
                let r: ConfigResult = res.unwrap();
                match r {
                    ConfigResult::Data {
                        value: content,
                        md5,
                        config_type,
                        last_modified,
                        ..
                    } => {
                        //v.to_owned()
                        response.result_code = SUCCESS_CODE;
                        response.content = content;
                        response.content_type =
                            Some(config_type.unwrap_or(ConfigType::Text.get_value()));
                        //response.encrypted_data_key = Some("".to_owned());
                        //java nacos中定义tag类型是String;
                        //nacos-sdk-go中定义tag类型为bool, nacos-sdk-go中直接设置 response.tag = request.tag会报错
                        if let Some(tag) = request.tag {
                            if !tag.is_empty() {
                                response.tag = Some(tag);
                            }
                        }
                        if request_meta.client_version.client.is_go_sdk() {
                            response.tag = None;
                        }
                        response.last_modified = last_modified;
                        response.md5 = Some(md5);
                    }
                    _ => {
                        response.result_code = ERROR_CODE;
                        response.error_code = NOT_FOUND;
                        response.message = Some("config data not exist".to_owned());
                    }
                }
                Ok(HandlerResult::success(PayloadUtils::build_payload(
                    "ConfigQueryResponse",
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
