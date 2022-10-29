pub mod nacos_proto;
pub mod server;

pub fn build_payload(val:&str) -> nacos_proto::Payload {
    let body = nacos_proto::Any {
        type_url:"".into(),
        value:val.as_bytes().to_vec(),
    };
    nacos_proto::Payload{
        body:Some(body),
        metadata:Default::default(),
    }
}

pub fn get_payload_string(value:&nacos_proto::Payload) -> String {
    let mut str = String::default();
    if let Some(meta) = &value.metadata {
        str.push_str(&format!("type:{},\n\t",meta.r#type));
        str.push_str(&format!("client_ip:{},\n\t",meta.client_ip));
        str.push_str(&format!("header:{:?},\n\t",meta.headers));
    }
    if let Some(body) = &value.body {
        let new_value = body.clone();
        let value_str = String::from_utf8(new_value.value).unwrap();
        str.push_str(&format!("body:{}",value_str));
    }
    str
}