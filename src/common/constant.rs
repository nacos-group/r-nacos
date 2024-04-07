use std::sync::Arc;

pub const APP_VERSION: &str = "0.5.4";

pub const EMPTY_STR: &str = "";

pub const HTTP_METHOD_GET: &str = "GET";
//pub const HTTP_METHOD_PUT:&str= "PUT";
//pub const HTTP_METHOD_POST:&str= "POST";
//pub const HTTP_METHOD_DELETE:&str= "DELETE";
pub const HTTP_METHOD_ALL: &str = EMPTY_STR;

pub const SEQ_KEY_CONFIG: &str = "SEQ_CONFIG";

lazy_static::lazy_static! {
    pub static ref CONFIG_TREE_NAME: Arc<String> =  Arc::new("T_CONFIG".to_string());
    pub static ref SEQUENCE_TREE_NAME: Arc<String> =  Arc::new("T_SEQUENCE".to_string());
    pub static ref USER_TREE_NAME: Arc<String> =  Arc::new("T_USER".to_string());
    pub static ref CACHE_TREE_NAME: Arc<String> =  Arc::new("T_CACHE".to_string());
}
