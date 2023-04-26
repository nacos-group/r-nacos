#![allow(unused_imports)]

use std::sync::Arc;

use crate::{grpc::{PayloadHandler, api_model::{ConfigPublishRequest, BaseResponse, ConfigQueryRequest, ConfigQueryResponse, ConfigBatchListenRequest, ConfigChangeBatchListenResponse, SUCCESS_CODE, ERROR_CODE, ConfigContext}, nacos_proto::Payload, PayloadUtils}, config::config::{ConfigActor, ConfigCmd, ConfigKey, ConfigResult, ListenerItem}};
use actix::prelude::Addr;
use async_trait::async_trait;

pub struct ConfigChangeBatchListenRequestHandler{
    config_addr: Addr<ConfigActor>,
}

impl ConfigChangeBatchListenRequestHandler {
    pub fn new(config_addr: Addr<ConfigActor>) -> Self {
        Self { config_addr }
    }
}

#[async_trait]
impl PayloadHandler for ConfigChangeBatchListenRequestHandler {
    async fn handle(&self, request_payload: crate::grpc::nacos_proto::Payload,request_meta:crate::grpc::RequestMeta) -> anyhow::Result<Payload> {
        let body_vec = request_payload.body.unwrap_or_default().value;
        let request:ConfigBatchListenRequest = serde_json::from_slice(&body_vec)?;
        let mut listener_items = vec![];
        for item in request.config_listen_contexts {
            let key = ConfigKey::new(&item.data_id, &item.group, &item.tenant);
            listener_items.push(ListenerItem::new(key, item.md5));
        }
        let cmd = ConfigCmd::Subscribe(listener_items,request_meta.connection_id);
        let mut response = ConfigChangeBatchListenResponse::default();
        match self.config_addr.send(cmd).await{
            Ok(res) => {
                let r:ConfigResult = res.unwrap();
                match r {
                    ConfigResult::ChangeKey(keys) => {
                        response.result_code = SUCCESS_CODE;
                        for key in keys {
                            let mut obj = ConfigContext::default();
                            obj.data_id = key.data_id;
                            obj.group = key.group;
                            obj.tenant = key.tenant;
                            response.changed_configs.push(obj);
                        }
                    },
                    _ => {
                        response.result_code = SUCCESS_CODE;
                    }
                }
            },
            Err(err) => {
                response.result_code = ERROR_CODE;
                response.error_code = ERROR_CODE;
                response.message = Some(err.to_string());
            }
        };
        Ok(PayloadUtils::build_payload("ConfigChangeBatchListenResponse", serde_json::to_string(&response)?))
    }
}
