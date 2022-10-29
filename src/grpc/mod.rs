use std::{collections::HashMap};

use self::api_model::BaseResponse;

pub mod handler;
pub mod nacos_proto;
pub mod server;
pub mod api_model;
pub mod bistream_manage;
pub mod bistream_conn;

pub trait PayloadHandler {
    fn handle(&self, request_payload: nacos_proto::Payload) -> nacos_proto::Payload;
}

pub struct PayloadUtils;

impl PayloadUtils {
    pub fn new_metadata(r#type:&str,client_ip:&str,headers:HashMap<String,String>) -> nacos_proto::Metadata {
        nacos_proto::Metadata {
            r#type:r#type.to_owned(),
            client_ip:client_ip.to_owned(),
            headers
        }
    }

    pub fn build_error_payload(error_code:u16,error_msg:String) -> nacos_proto::Payload {
        let error_val = BaseResponse::build_error_response(error_code,error_msg).to_json_string();
        Self::build_payload("ErrorResponse",error_val)
    }

    pub fn build_payload(url:&str,val: String) -> nacos_proto::Payload {
        Self::build_full_payload(url,val,"",Default::default())
    }

    pub fn build_full_payload(url:&str,val: String,client_ip:&str,headers:HashMap<String,String>) -> nacos_proto::Payload {
        let body = nacos_proto::Any {
            type_url: "".into(),
            value: val.into_bytes(),
        };
        let meta = Self::new_metadata(url,client_ip,headers);
        nacos_proto::Payload {
            body: Some(body),
            metadata: Some(meta),
        }
    }

    pub fn get_payload_string(payload: &nacos_proto::Payload) -> String {
        let mut str = String::default();
        if let Some(meta) = &payload.metadata {
            str.push_str(&format!("type:{},\n\t", meta.r#type));
            str.push_str(&format!("client_ip:{},\n\t", meta.client_ip));
            str.push_str(&format!("header:{:?},\n\t", meta.headers));
        }
        if let Some(body) = &payload.body {
            let new_value = body.clone();
            let value_str = String::from_utf8(new_value.value).unwrap();
            str.push_str(&format!("body:{}", value_str));
        }
        str
    }

    pub fn get_payload_type<'a>(payload: &'a nacos_proto::Payload) -> Option<&'a String> {
        if let Some(meta) = &payload.metadata {
            return Some(&meta.r#type);
        }
        None
    }
}
