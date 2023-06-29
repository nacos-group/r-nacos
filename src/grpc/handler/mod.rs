use crate::{config::core::ConfigActor, naming::core::NamingActor};

use self::{config_publish::ConfigPublishRequestHandler, config_query::ConfigQueryRequestHandler, config_remove::ConfigRemoveRequestHandler, config_change_batch_listen::ConfigChangeBatchListenRequestHandler, naming_instance::InstanceRequestHandler, naming_subscribe_service::SubscribeServiceRequestHandler, naming_service_query::ServiceQueryRequestHandler, naming_batch_instance::BatchInstanceRequestHandler};

use super::{PayloadHandler, PayloadUtils, api_model::{ServerCheckResponse, SUCCESS_CODE, BaseResponse}, RequestMeta, nacos_proto::Payload};
use actix::Addr;
use async_trait::async_trait;

pub mod config_publish;
pub mod config_remove;
pub mod config_query;
pub mod config_change_batch_listen;

pub mod naming_instance;
pub mod naming_batch_instance;
pub mod naming_subscribe_service;
pub mod naming_service_query;
pub mod converter;


#[derive(Default)]
pub struct InvokerHandler{
    handlers:Vec<(String,Box<dyn PayloadHandler + Send + Sync + 'static>)>,
}
pub struct HealthCheckRequestHandler{}

impl InvokerHandler {
    pub fn new() -> Self {
        let mut this=Self { 
            handlers:Default::default(),
        };
        this.add_handler("HealthCheckRequest", Box::new(HealthCheckRequestHandler{}));
        this
    }

    pub fn add_handler(&mut self,url:&str,handler:Box<dyn PayloadHandler+ Send + Sync +'static>){
        self.handlers.push((url.to_owned(),handler));
    }

    pub fn match_handler<'a>(&'a self,url:&str) -> Option<&'a (dyn PayloadHandler+ Send + Sync +'static)> {
        for (t,h) in &self.handlers {
            if t==url {
                return Some(h.as_ref())
            }
        }
        None
    }

    pub fn add_config_handler(&mut self,config_addr:&Addr<ConfigActor>) {
        self.add_handler("ConfigQueryRequest", Box::new(ConfigQueryRequestHandler::new(config_addr.clone())));
        self.add_handler("ConfigPublishRequest", Box::new(ConfigPublishRequestHandler::new(config_addr.clone())));
        self.add_handler("ConfigRemoveRequest", Box::new(ConfigRemoveRequestHandler::new(config_addr.clone())));
        self.add_handler("ConfigBatchListenRequest", Box::new(ConfigChangeBatchListenRequestHandler::new(config_addr.clone())));
    }

    pub fn add_naming_handler(&mut self,naming_addr:&Addr<NamingActor>) {
        self.add_handler("InstanceRequest", Box::new(InstanceRequestHandler::new(naming_addr.clone())));
        self.add_handler("BatchInstanceRequest", Box::new(BatchInstanceRequestHandler::new(naming_addr.clone())));
        self.add_handler("SubscribeServiceRequest", Box::new(SubscribeServiceRequestHandler::new(naming_addr.clone())));
        self.add_handler("ServiceQueryRequest", Box::new(ServiceQueryRequestHandler::new(naming_addr.clone())));
    }
}



#[async_trait]
impl PayloadHandler for InvokerHandler {
    async fn handle(&self,request_payload:super::nacos_proto::Payload,request_meta:RequestMeta) -> anyhow::Result<Payload> {
        if let Some(url) = PayloadUtils::get_payload_type(&request_payload) {
            if "ServerCheckRequest"==url {
                let response = ServerCheckResponse{
                    result_code : SUCCESS_CODE,
                    connection_id : Some(request_meta.connection_id.as_ref().to_owned()),
                    ..Default::default()
                };
                return Ok(PayloadUtils::build_payload("ServerCheckResponse", serde_json::to_string(&response)?))
            }
            //println!("InvokerHandler type:{}",url);
            if let Some(handler) = self.match_handler(url) {
                return handler.handle(request_payload,request_meta).await;
            }
            log::warn!("InvokerHandler not fund handler,type:{}",url);
            return Ok(PayloadUtils::build_error_payload(302u16,format!("{} RequestHandler Not Found",url)))
        }
        Ok(PayloadUtils::build_error_payload(302u16,"empty type url".to_owned()))
    }
}


#[async_trait]
impl PayloadHandler for HealthCheckRequestHandler {
    async fn handle(&self, _request_payload: super::nacos_proto::Payload,_request_meta:RequestMeta) -> anyhow::Result<Payload> {
        //println!("HealthCheckRequest");
        let response = BaseResponse::build_success_response();
        return Ok(PayloadUtils::build_payload("HealthCheckResponse", serde_json::to_string(&response)?))
    }
}