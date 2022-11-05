use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub const SUCCESS_CODE:u16= 200u16;
pub const ERROR_CODE:u16= 500u16;

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BaseResponse{
    pub result_code:u16,
    pub error_code:u16,
    pub message:Option<String>,
    pub request_id:Option<String>,
}

pub type ErrorResponse= BaseResponse;

impl BaseResponse {

    pub fn build_success_response() -> Self {
        Self { 
            result_code: SUCCESS_CODE,
            error_code:0,
            message: None,
            request_id: None,
        }
    }

    pub fn build_error_response(error_code:u16,error_msg:String) -> Self {
        Self { 
            result_code: ERROR_CODE,
            error_code,
            message: Some(error_msg),
            request_id: None,
        }
    }

    pub fn to_json_string(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }
}


#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ServerCheckResponse{
    pub result_code:u16,
    pub error_code:u16,
    pub message:Option<String>,
    pub request_id:Option<String>,
    pub connection_id:Option<String>,
}


#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ClientDetectionRequest {
    pub module:Option<String>,
    pub request_id:Option<String>,
    pub headers:HashMap<String,String>,
}


// --- config ---

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConfigPublishRequest {
    pub module:Option<String>,
    pub request_id:Option<String>,
    pub headers:HashMap<String,String>,
    pub data_id: String,
    pub group: String,
    pub tenant: String,
    pub content: String,
    pub cas_md5: Option<String>,
    pub addition_map:HashMap<String,String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConfigQueryRequest {
    pub module:Option<String>,
    pub request_id:Option<String>,
    pub headers:HashMap<String,String>,
    pub data_id: String,
    pub group: String,
    pub tenant: String,
    pub tag: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConfigQueryResponse{
    pub result_code:u16,
    pub error_code:u16,
    pub message:Option<String>,
    pub request_id:Option<String>,

    pub content:String,
    pub encrypted_data_key:Option<String>,
    pub content_type:Option<String>,
    pub md5:Option<String>,
    pub last_modified:u64,
    pub beta:bool,
    pub tag: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConfigRemoveRequest {
    pub module:Option<String>,
    pub request_id:Option<String>,
    pub headers:HashMap<String,String>,

    pub data_id: String,
    pub group: String,
    pub tenant: String,
    pub tag: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConfigListenContext {
    pub data_id: String,
    pub group: String,
    pub tenant: String,
    pub md5: String,
    pub tag: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConfigBatchListenRequest {
    pub module:Option<String>,
    pub request_id:Option<String>,
    pub headers:HashMap<String,String>,

    pub listen: bool,
    pub config_listen_contexts: Vec<ConfigListenContext>,

}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConfigContext {
    pub data_id: String,
    pub group: String,
    pub tenant: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConfigChangeBatchListenResponse{
    pub result_code:u16,
    pub error_code:u16,
    pub message:Option<String>,
    pub request_id:Option<String>,

    pub changed_configs: Vec<ConfigContext>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConfigChangeNotifyRequest {
    pub module:Option<String>,
    pub request_id:Option<String>,
    pub headers:HashMap<String,String>,

    pub data_id: String,
    pub group: String,
    pub tenant: String,
}

// ----- naming model -----

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ApiInstance{
    pub instance_id:Option<String>,
    pub ip:Option<String>,
    pub port:u32,
    pub weight:f32,
    pub healthy:bool,
    pub enabled:bool,
    pub ephemeral: bool,
    pub cluster_name:Option<String>,
    pub service_name:Option<String>,
    pub metadata:HashMap<String,String>,
    pub instance_heart_beat_interval:i64,
    pub instance_heart_beat_time_out:i64,
    pub ip_delete_timeout:i64,
    pub instance_id_generator:Option<String>
}


#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InstanceRequest{
    pub module:Option<String>,
    pub request_id:Option<String>,
    pub headers:HashMap<String,String>,

    pub namespace:Option<String>,
    pub service_name:Option<String>,
    pub group_name:Option<String>,

    pub r#type:Option<String>,
    pub instance: Option<ApiInstance>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InstanceResponse {
    pub result_code:u16,
    pub error_code:u16,
    pub message:Option<String>,
    pub request_id:Option<String>,

    pub r#type:Option<String>,
}