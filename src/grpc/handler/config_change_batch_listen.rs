#![allow(unused_imports)]

use crate::{grpc::{PayloadHandler, api_model::{ConfigPublishRequest, BaseResponse, ConfigQueryRequest, ConfigQueryResponse, ConfigBatchListenRequest, ConfigChangeBatchListenResponse, SUCCESS_CODE}, nacos_proto::Payload, PayloadUtils}, config::config::{ConfigActor, ConfigCmd, ConfigKey, ConfigResult}};
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
    async fn handle(&self, request_payload: crate::grpc::nacos_proto::Payload,_request_meta:crate::grpc::RequestMeta) -> anyhow::Result<Payload> {
        let body_vec = request_payload.body.unwrap_or_default().value;
        let _request:ConfigBatchListenRequest = serde_json::from_slice(&body_vec)?;
        let mut response = ConfigChangeBatchListenResponse::default();
        response.result_code = SUCCESS_CODE;
        let response_str = serde_json::to_string(&response)?;
        /*
        let cmd = ConfigCmd::LISTENER(ConfigKey::new(&request.data_id,&request.group,&request.tenant));
        let response_str = match self.config_addr.send(cmd).await{
            Ok(res) => {
                //let res:ConfigResult = res.unwrap();
                let r:ConfigResult = res.unwrap();
                match r {
                    ConfigResult::DATA(content) => {
                        //v.to_owned()
                        let mut response = ConfigQueryResponse::default();
                        response.content = content;
                        response.tag = request.tag;
                        serde_json::to_string(&response)?
                    },
                    ConfigResult::NULL => {
                        //"config data not exist".to_owned()
                        serde_json::to_string(&BaseResponse::build_error_response(500u16,"config data not exist".to_owned()))?
                    }
                }
            },
            Err(err) => {
                serde_json::to_string(&BaseResponse::build_error_response(500u16,err.to_string()))?
            }
        };
        */
        Ok(PayloadUtils::build_payload("ConfigChangeBatchListenResponse", response_str))
    }
}
