use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};

pub const SUCCESS_CODE: u16 = 200u16;
pub const NOT_FOUND: u16 = 300u16;
pub const ERROR_CODE: u16 = 500u16;

pub const INTERNAL_MODEL: &str = "internal";
pub const CONFIG_MODEL: &str = "config";
pub const NAMING_MODEL: &str = "naming";

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BaseResponse {
    pub result_code: u16,
    pub error_code: u16,
    pub message: Option<String>,
    pub request_id: Option<String>,
}

pub type ErrorResponse = BaseResponse;

impl BaseResponse {
    pub fn build_success_response() -> Self {
        Self {
            result_code: SUCCESS_CODE,
            error_code: 0,
            message: Some("".to_string()),
            request_id: Some("".to_string()),
        }
    }

    pub fn build_error_response(error_code: u16, error_msg: String) -> Self {
        Self {
            result_code: ERROR_CODE,
            error_code,
            message: Some(error_msg),
            request_id: Some("".to_string()),
        }
    }

    pub fn to_json_string(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ServerCheckResponse {
    pub result_code: u16,
    pub error_code: u16,
    pub message: String,
    pub request_id: String,
    pub connection_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ClientDetectionRequest {
    pub module: Option<String>,
    pub request_id: Option<String>,
    pub headers: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConnectResetRequest {
    pub module: Option<String>,
    pub request_id: Option<String>,
    pub headers: Option<HashMap<String, String>>,

    pub server_ip: Option<String>,
    pub server_port: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionSetupRequest {
    pub module: Option<String>,
    pub request_id: Option<String>,
    pub headers: Option<HashMap<String, String>>,

    pub client_version: Option<String>,
    pub tenant: Option<String>,
    pub labels: Option<HashMap<String, String>>,
}

// --- config ---

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConfigPublishRequest {
    pub module: Option<String>,
    pub request_id: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    pub data_id: String,
    pub group: String,
    pub tenant: String,
    pub content: Arc<String>,
    pub cas_md5: Option<String>,
    pub addition_map: Option<HashMap<String, Option<String>>>,
}

impl ConfigPublishRequest {
    pub fn get_addition_param(&self, key: &str) -> Option<&String> {
        if let Some(addition_map) = self.addition_map.as_ref() {
            if let Some(v) = addition_map.get(key) {
                v.as_ref()
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConfigQueryRequest {
    pub module: Option<String>,
    pub request_id: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    pub data_id: String,
    pub group: String,
    pub tenant: String,
    pub tag: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConfigQueryResponse {
    pub result_code: u16,
    pub error_code: u16,
    pub message: Option<String>,
    pub request_id: Option<String>,

    pub content: Arc<String>,
    pub encrypted_data_key: Option<String>,
    pub content_type: Option<Arc<String>>,
    pub md5: Option<Arc<String>>,
    pub last_modified: i64,
    pub beta: bool,
    pub tag: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConfigRemoveRequest {
    pub module: Option<String>,
    pub request_id: Option<String>,
    pub headers: Option<HashMap<String, String>>,

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
    pub md5: Arc<String>,
    pub tag: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConfigBatchListenRequest {
    pub module: Option<String>,
    pub request_id: Option<String>,
    pub headers: Option<HashMap<String, String>>,

    pub listen: bool,
    pub config_listen_contexts: Vec<ConfigListenContext>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConfigContext {
    pub data_id: Arc<String>,
    pub group: Arc<String>,
    pub tenant: Arc<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConfigChangeBatchListenResponse {
    pub result_code: u16,
    pub error_code: u16,
    pub message: Option<String>,
    pub request_id: Option<String>,

    pub changed_configs: Vec<ConfigContext>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConfigChangeNotifyRequest {
    pub module: Option<String>,
    pub request_id: Option<String>,
    pub headers: HashMap<String, String>,

    pub data_id: Arc<String>,
    pub group: Arc<String>,
    pub tenant: Arc<String>,
}

// ----- naming model -----

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    pub instance_id: Option<Arc<String>>,
    pub ip: Option<Arc<String>>,
    pub port: u32,
    pub weight: f32,
    pub healthy: bool,
    pub enabled: bool,
    pub ephemeral: bool,
    pub cluster_name: Option<String>,
    pub service_name: Option<Arc<String>>,
    pub metadata: Option<Arc<HashMap<String, String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub instance_heart_beat_interval: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub instance_heart_beat_time_out: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub ip_delete_timeout: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub instance_id_generator: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InstanceRequest {
    pub module: Option<String>,
    pub request_id: Option<String>,
    pub headers: Option<HashMap<String, String>>,

    pub namespace: Option<String>,
    pub service_name: Option<String>,
    pub group_name: Option<String>,

    pub r#type: Option<String>,
    pub instance: Option<Instance>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InstanceResponse {
    pub result_code: u16,
    pub error_code: u16,
    pub message: Option<String>,
    pub request_id: Option<String>,

    pub r#type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeServiceRequest {
    pub module: Option<String>,
    pub request_id: Option<String>,
    pub headers: Option<HashMap<String, String>>,

    pub namespace: Option<String>,
    pub service_name: Option<String>,
    pub group_name: Option<String>,

    pub subscribe: bool,
    pub clusters: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ServiceInfo {
    pub name: Option<Arc<String>>,
    pub group_name: Option<Arc<String>>,
    pub clusters: Option<String>,
    pub cache_millis: i64,
    pub hosts: Option<Vec<Instance>>,
    pub last_ref_time: i64,
    pub checksum: Option<String>,
    #[serde(rename = "allIPs")]
    pub all_ips: bool,
    pub reach_protection_threshold: bool,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeServiceResponse {
    pub result_code: u16,
    pub error_code: u16,
    pub message: Option<String>,
    pub request_id: Option<String>,

    pub service_info: Option<ServiceInfo>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BatchInstanceRequest {
    pub module: Option<String>,
    pub request_id: Option<String>,
    pub headers: Option<HashMap<String, String>>,

    pub namespace: Option<String>,
    pub service_name: Option<String>,
    pub group_name: Option<String>,

    pub r#type: Option<String>,
    pub instances: Option<Vec<Instance>>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BatchInstanceResponse {
    pub result_code: u16,
    pub error_code: u16,
    pub message: Option<String>,
    pub request_id: Option<String>,

    pub r#type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ServiceQueryRequest {
    pub module: Option<String>,
    pub request_id: Option<String>,
    pub headers: Option<HashMap<String, String>>,

    pub namespace: Option<String>,
    pub service_name: Option<String>,
    pub group_name: Option<String>,

    pub cluster: Option<String>,
    pub healthy_only: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ServiceQueryResponse {
    pub result_code: u16,
    pub error_code: u16,
    pub message: Option<String>,
    pub request_id: Option<String>,

    pub service_info: Option<ServiceInfo>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NotifySubscriberRequest {
    pub module: Option<String>,
    pub request_id: Option<String>,
    pub headers: HashMap<String, String>,

    pub namespace: Option<Arc<String>>,
    pub service_name: Option<Arc<String>>,
    pub group_name: Option<Arc<String>>,

    pub service_info: Option<ServiceInfo>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ServiceListRequest {
    pub module: Option<String>,
    pub request_id: Option<String>,
    pub headers: Option<HashMap<String, String>>,

    pub namespace: Option<String>,
    pub service_name: Option<String>,
    pub group_name: Option<String>,

    pub page_no: u32,
    pub page_size: u32,
    pub selector: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ServiceListResponse {
    pub result_code: u16,
    pub error_code: u16,
    pub message: Option<String>,
    pub request_id: Option<String>,

    pub count: usize,
    pub service_names: Option<Vec<Arc<String>>>,
}
