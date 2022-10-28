use super::core::{Instance, NamingUtils, ServiceKey};
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct QueryListResult {
    pub name: String,
    pub clusters: String,
    pub cache_millis: u64,
    pub hosts: Vec<InstanceVO>,
    pub last_ref_time: Option<i64>,
    pub checksum: Option<String>,
    #[serde(rename = "useSpecifiedURL")]
    pub use_specified_url: Option<bool>,
    pub env: Option<String>,
    pub protect_threshold: Option<f32>,
    pub reach_local_site_call_threshold: Option<bool>,
    pub dom: Option<String>,
    pub metadata: Option<HashMap<String, String>>,
}

impl QueryListResult {
    pub fn get_instance_list_string(
        clusters: String,
        key: &ServiceKey,
        v: Vec<Instance>,
    ) -> String {
        let mut result = QueryListResult::default();
        result.name = key.get_join_service_name();
        result.cache_millis = 10000u64;
        let now = Local::now().timestamp_millis();
        result.last_ref_time = Some(now);
        result.checksum = Some(now.to_string());
        result.use_specified_url = Some(false);
        result.clusters = clusters;
        result.env = Some("".to_owned());
        result.hosts = v
            .into_iter()
            .map(|e| InstanceVO::from_instance(&e))
            .collect::<Vec<_>>();
        result.dom = Some(key.service_name.to_owned());
        serde_json::to_string(&result).unwrap()
    }

    pub fn get_ref_instance_list_string(
        clusters: String,
        key: &ServiceKey,
        v: Vec<&Instance>,
    ) -> String {
        let mut result = QueryListResult::default();
        result.name = key.get_join_service_name();
        result.cache_millis = 10000u64;
        let now = Local::now().timestamp_millis();
        result.last_ref_time = Some(now - 1000);
        result.checksum = Some(now.to_string());
        result.use_specified_url = Some(false);
        result.clusters = clusters;
        result.env = Some("".to_owned());
        result.hosts = v
            .into_iter()
            .map(|e| InstanceVO::from_instance(&e))
            .collect::<Vec<_>>();
        result.dom = Some(key.service_name.to_owned());
        serde_json::to_string(&result).unwrap()
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InstanceVO {
    pub service: String,
    pub ip: String,
    pub port: u32,
    pub cluster_name: String,
    pub weight: f32,
    pub healthy: bool,
    pub instance_id: String,
    pub metadata: HashMap<String, String>,
    pub marked: Option<bool>,
    pub enabled: Option<bool>,
    pub service_name: Option<String>,
    pub ephemeral: Option<bool>,
}

impl InstanceVO {
    pub fn from_instance(instance: &Instance) -> Self {
        Self {
            service: NamingUtils::get_group_and_service_name(
                &instance.service_name,
                &instance.group_name,
            ),
            ip: instance.ip.to_owned(),
            port: instance.port,
            cluster_name: instance.cluster_name.to_owned(),
            weight: instance.weight,
            healthy: instance.healthy,
            instance_id: instance.id.to_owned(),
            metadata: instance.metadata.clone(),
            marked: Some(true),
            enabled: Some(instance.enabled),
            service_name: Some(instance.service_name.to_owned()),
            ephemeral: Some(instance.ephemeral),
        }
    }
}
