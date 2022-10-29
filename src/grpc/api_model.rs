use serde::{Deserialize, Serialize};

pub const SUCCESS_CODE:u16= 200u16;
pub const ERROR_CODE:u16= 500u16;

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BaseResponse{
    result_code:u16,
    error_code:u16,
    message:Option<String>,
    request_id:Option<String>,
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