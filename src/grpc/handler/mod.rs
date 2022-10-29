use std::collections::HashMap;

use super::{PayloadHandler, PayloadUtils, api_model::{ServerCheckResponse, SUCCESS_CODE, BaseResponse}};


#[derive(Default)]
pub struct RequestMeta{
    pub connection_id:String,
    pub client_ip:String,
    pub client_version:String,
    pub labels:HashMap<String,String>,
}

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

    pub fn match_handler<'a>(&'a self,url:&str) -> Option<&'a Box<dyn PayloadHandler+ Send + Sync +'static>> {
        for (t,h) in &self.handlers {
            if t==url {
                return Some(h)
            }
        }
        None
    }
}



impl PayloadHandler for InvokerHandler {
    fn handle(&self,request_payload:super::nacos_proto::Payload,request_meta:RequestMeta) -> super::nacos_proto::Payload {
        if let Some(url) = PayloadUtils::get_payload_type(&request_payload) {
            if "ServerCheckRequest"==url {
                let mut response = ServerCheckResponse::default();
                response.result_code = SUCCESS_CODE;
                response.connection_id = Some(request_meta.connection_id.to_owned());
                return PayloadUtils::build_payload("ServerCheckResponse", serde_json::to_string(&response).unwrap())
            }
            if let Some(handler) = self.match_handler(url) {
                return handler.handle(request_payload,request_meta);
            }
        }
        PayloadUtils::build_error_payload(302u16,"RequestHandler Not Found".to_owned())
    }
}


impl PayloadHandler for HealthCheckRequestHandler {
    fn handle(&self, request_payload: super::nacos_proto::Payload,request_meta:RequestMeta) -> super::nacos_proto::Payload {
        println!("HealthCheckRequest");
        let response = BaseResponse::build_success_response();
        return PayloadUtils::build_payload("HealthCheckResponse", serde_json::to_string(&response).unwrap())
    }
}