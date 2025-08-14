use crate::common::model::ClientVersion;
use std::sync::Arc;

pub const EMPTY_STR: &str = "";

pub const HTTP_METHOD_GET: &str = "GET";
//pub const HTTP_METHOD_PUT:&str= "PUT";
pub const HTTP_METHOD_POST: &str = "POST";
// pub const HTTP_METHOD_DELETE:&str= "DELETE";
pub const HTTP_METHOD_ALL: &str = EMPTY_STR;

pub const SEQ_KEY_CONFIG: &str = "SEQ_CONFIG";

pub const AUTHORIZATION_HEADER: &str = "Authorization";
pub const ACCESS_TOKEN_HEADER: &str = "accessToken";

pub const GRPC_HEAD_KEY_CLUSTER_ID: &str = "cluster_id";
pub const GRPC_HEAD_KEY_TRACE_ID: &str = "trace_id";

pub const GRPC_HEAD_KEY_SUB_NAME: &str = "sub_name";

lazy_static::lazy_static! {
    pub static ref CONFIG_TREE_NAME: Arc<String> =  Arc::new("T_CONFIG".to_string());
    pub static ref SEQUENCE_TREE_NAME: Arc<String> =  Arc::new("T_SEQUENCE".to_string());
    pub static ref USER_TREE_NAME: Arc<String> =  Arc::new("T_USER".to_string());
    pub static ref CACHE_TREE_NAME: Arc<String> =  Arc::new("T_CACHE".to_string());
    pub static ref NAMESPACE_TREE_NAME: Arc<String> =  Arc::new("T_NAMESPACE".to_string());
    pub static ref MCP_SERVER_TABLE_NAME: Arc<String> =  Arc::new("T_MCP_SERVER".to_string());
    pub static ref MCP_TOOL_SPEC_TABLE_NAME: Arc<String> =  Arc::new("T_MCP_TOOL_SPEC".to_string());
    pub static ref EMPTY_ARC_STRING: Arc<String> = Arc::new("".to_string());
    pub static ref SEQ_TOOL_SPEC_VERSION: Arc<String> =  Arc::new("TOOL_SPEC_VERSION".to_string());
    pub static ref SEQ_MCP_SERVER_ID: Arc<String> =  Arc::new("MCP_SERVER_ID".to_string());
    pub static ref SEQ_MCP_SERVER_VALUE_ID: Arc<String> =  Arc::new("MCP_SERVER_VALUE_ID".to_string());
    pub static ref DEFAULT_NAMESPACE_ARC_STRING: Arc<String> = Arc::new("".to_string());
    pub static ref EMPTY_CLIENT_VERSION: Arc<ClientVersion> = Arc::new(ClientVersion::default());
}
