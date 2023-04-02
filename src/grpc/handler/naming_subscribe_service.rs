#![allow(unused_imports)]

use std::sync::Arc;

use crate::{
    grpc::{PayloadHandler, 
        api_model::{SUCCESS_CODE, ERROR_CODE, Instance as ApiInstance,ServiceInfo as ApiServiceInfo, SubscribeServiceRequest, SubscribeServiceResponse}, 
        nacos_proto::Payload, PayloadUtils
    }, 
    naming::{core::{NamingActor, NamingCmd, NamingResult}, 
    model::{Instance, ServiceKey, ServiceInfo}, NamingUtils, naming_subscriber::NamingListenerItem}, 
    now_millis_i64
};
use actix::prelude::Addr;
use async_trait::async_trait;

use super::converter::ModelConverter;

pub struct SubscribeServiceRequestHandler{
    naming_addr: Addr<NamingActor>,
}

impl SubscribeServiceRequestHandler {
    pub fn new(naming_addr: Addr<NamingActor>) -> Self {
        Self { naming_addr }
    }

    fn convert_to_service_info(&self,info:ServiceInfo) -> ApiServiceInfo {
        ModelConverter::to_api_service_info(info)
    }

    fn parse_clusters(&self,request:&SubscribeServiceRequest) -> Vec<String> {
        let mut clusters = vec![];
        if let Some(cluster_str) = request.clusters.as_ref() {
            clusters = cluster_str.split(",").into_iter()
                .filter(|e|{e.len()>0}).map(|e|{e.to_owned()}).collect::<Vec<_>>();
        }
        clusters
    }

    fn build_subscribe_cmd(&self,subscribe:bool,service_key:ServiceKey,connection_id:Arc<String>) -> NamingCmd {
        let item = NamingListenerItem {
            service_key,
            clusters:None,
        };
        if subscribe {
            NamingCmd::Subscribe(vec![item], connection_id)
        }
        else{
            NamingCmd::RemoveSubscribe(vec![item], connection_id)
        }
    }
}

#[async_trait]
impl PayloadHandler for SubscribeServiceRequestHandler {
    async fn handle(&self, request_payload: crate::grpc::nacos_proto::Payload,request_meta:crate::grpc::RequestMeta) -> anyhow::Result<Payload> {
        let body_vec = request_payload.body.unwrap_or_default().value;
        let request:SubscribeServiceRequest = serde_json::from_slice(&body_vec)?;
        let mut response = SubscribeServiceResponse::default();
        let clusters = self.parse_clusters(&request);
        let namespace = request.namespace.as_ref().unwrap_or(&"public".to_owned()).to_owned();
        let key = ServiceKey::new(&namespace
            ,&request.group_name.unwrap_or_default(),&request.service_name.unwrap_or_default());
        let subscribe_cmd = self.build_subscribe_cmd(request.subscribe, key.clone(), request_meta.connection_id.clone());
        self.naming_addr.do_send(subscribe_cmd);
        let cmd = NamingCmd::QueryServiceInfo(key,clusters,true);
        match self.naming_addr.send(cmd).await{
            Ok(res) => {
                let result: NamingResult = res.unwrap();
                match result {
                    NamingResult::ServiceInfo(service_info) => {
                        let api_service_info = self.convert_to_service_info(service_info);
                        response.service_info = Some(api_service_info);
                        response.result_code = SUCCESS_CODE;
                    },
                    _ => {
                        response.result_code = ERROR_CODE;
                        response.error_code = 500u16;
                        response.message = Some("naming handler result type is error".to_owned());
                    }
                };
            },
            Err(err) => {
                response.result_code = ERROR_CODE;
                response.error_code = 500u16;
                response.message = Some(err.to_string());
            }
        };
        Ok(PayloadUtils::build_payload("SubscribeServiceResponse", serde_json::to_string(&response)?))
    }

}
