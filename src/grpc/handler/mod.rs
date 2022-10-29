use super::{PayloadHandler, PayloadUtils};



#[derive(Default)]
pub struct InvokerHandler{
    handlers:Vec<(String,Box<dyn PayloadHandler + Send + Sync + 'static>)>,
}

impl InvokerHandler {
    pub fn new() -> Self {
        Self { 
            handlers:Default::default(),
        }
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
    fn handle(&self,request_payload:super::nacos_proto::Payload) -> super::nacos_proto::Payload {
        if let Some(url) = PayloadUtils::get_payload_type(&request_payload) {
            if let Some(handler) = self.match_handler(url) {
                return handler.handle(request_payload);
            }
        }
        PayloadUtils::build_error_payload(302u16,"RequestHandler Not Found".to_owned())
    }
}