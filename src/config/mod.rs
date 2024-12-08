use crate::common::constant::EMPTY_ARC_STRING;
use std::sync::Arc;

pub mod config_db;
pub mod config_index;
pub mod config_sled;
pub mod config_subscribe;
pub mod config_type;
pub mod core;
pub mod dal;
pub mod metrics;
pub mod model;
pub mod utils;

pub struct ConfigUtils;

pub const DEFAULT_TENANT: &str = "public";

impl ConfigUtils {
    pub fn default_tenant(val: String) -> String {
        if val == DEFAULT_TENANT {
            "".to_owned()
        } else {
            val
        }
    }

    pub fn default_tenant_arc(val: Arc<String>) -> Arc<String> {
        if val.as_str() == DEFAULT_TENANT {
            EMPTY_ARC_STRING.clone()
        } else {
            val
        }
    }

    pub fn is_default_tenant(val: &str) -> bool {
        val == DEFAULT_TENANT
    }
}
