use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

// JSON-RPC 请求结构
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<Value>,
    pub id: Option<Value>,
}

// JSON-RPC 响应结构
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}

#[derive(Deserialize)]
pub struct McpPath {
    pub server_key: Arc<String>,
    pub auth_key: Arc<String>,
}

#[derive(Deserialize)]
pub struct SseMessagePath {
    pub node_id: u64,
    pub server_key: Arc<String>,
    pub session_id: Arc<String>,
}
