use std::collections::HashMap;

pub mod api_model;
pub mod core;
pub(crate) mod filter;
pub mod listener;
pub mod model;
pub mod naming_delay_nofity;
pub mod naming_subscriber;
pub mod service;
pub mod udp_actor;
//pub(crate) mod dal;
pub mod cluster;
pub mod metrics;
pub mod ops;
pub mod service_index;

#[cfg(feature = "debug")]
pub mod naming_debug;

pub struct NamingUtils;

pub const RESPONSE_CODE_KEY: &str = "code";
pub const RESPONSE_CODE_OK: i32 = 10200;
pub const CLIENT_BEAT_INTERVAL_KEY: &str = "clientBeatInterval";
pub const LIGHT_BEAT_ENABLED_KEY: &str = "lightBeatEnabled";

pub const DEFAULT_NAMESPACE: &str = "public";
pub const DEFAULT_CLUSTER: &str = "DEFAULT";
pub const DEFAULT_GROUP: &str = "DEFAULT_GROUP";

impl NamingUtils {
    pub fn get_group_and_service_name(service_name: &str, group_name: &str) -> String {
        format!("{}@@{}", group_name, service_name)
    }

    pub fn split_group_and_service_name(grouped_name: &str) -> Option<(String, String)> {
        let split = grouped_name.split("@@").collect::<Vec<_>>();
        if split.is_empty() {
            return None;
        }
        let a = split.first();
        let b = split.get(1);
        match b {
            Some(b) => {
                let a = a.unwrap();
                if a.is_empty() {
                    return None;
                }
                Some(((*a).to_owned(), (*b).to_owned()))
            }
            None => match a {
                Some(a) => {
                    if a.is_empty() {
                        return None;
                    }
                    Some(("DEFAULT_GROUP".to_owned(), (*a).to_owned()))
                }
                None => None,
            },
        }
    }

    pub fn split_filters(cluster_str: &str) -> Vec<String> {
        cluster_str
            .split(',')
            .filter(|e| !e.is_empty())
            .map(|e| e.to_owned())
            .collect::<Vec<_>>()
    }

    pub fn default_namespace(val: String) -> String {
        if val.is_empty() {
            DEFAULT_NAMESPACE.to_owned()
        } else {
            val
        }
    }

    pub fn default_cluster(val: String) -> String {
        if val.is_empty() {
            DEFAULT_CLUSTER.to_owned()
        } else {
            val
        }
    }

    pub fn default_group(val: String) -> String {
        if val.is_empty() {
            DEFAULT_GROUP.to_owned()
        } else {
            val
        }
    }

    ///
    /// 解析metadata
    /// 兼容支持json与nacos自定义格式
    pub fn parse_metadata(metadata_str: &str) -> anyhow::Result<HashMap<String, String>> {
        if metadata_str.is_empty() {
            return Err(anyhow::anyhow!("metadata is empty"));
        }
        if let Ok(metadata) = serde_json::from_str::<HashMap<String, String>>(metadata_str) {
            return Ok(metadata);
        }
        let mut metadata = HashMap::new();
        for item in metadata_str.split(',') {
            if item.is_empty() {
                continue;
            }
            let kv: Vec<&str> = item.split('=').collect();
            if kv.len() != 2 {
                return Err(anyhow::anyhow!(
                    "metadata format incorrect:{}",
                    &metadata_str
                ));
            }
            metadata.insert(
                kv.first().unwrap().to_string(),
                kv.get(1).unwrap().to_string(),
            );
        }
        Ok(metadata)
    }
}
