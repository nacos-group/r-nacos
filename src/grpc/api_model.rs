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
    pub is_beta:bool,
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