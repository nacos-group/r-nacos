
pub mod config;
pub mod api;
pub mod dal;
pub mod config_subscribe;
pub mod config_db;
pub mod config_index;

pub struct ConfigUtils;

pub const DEFAULT_TENANT: &str = "public";

impl ConfigUtils {
    pub fn default_tenant(val:String) -> String {
        if &val==DEFAULT_TENANT { "".to_owned() } else{ val }
    }
}