use crate::now_millis_i64;

use super::model::{Instance, ServiceDetailDto, ServiceKey};
use super::NamingUtils;
use crate::common::option_utils::OptionUtils;
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

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
    pub dom: Option<Arc<String>>,
    pub metadata: Option<HashMap<String, String>>,
}

impl QueryListResult {
    pub fn get_instance_list_string(
        clusters: String,
        key: &ServiceKey,
        v: Vec<Arc<Instance>>,
    ) -> String {
        let now = now_millis_i64();
        let result = Self {
            name: key.get_join_service_name(),
            cache_millis: 10000u64,
            last_ref_time: Some(now),
            checksum: Some(now.to_string()),
            use_specified_url: Some(false),
            clusters,
            env: Some("".to_owned()),
            hosts: v
                .into_iter()
                .map(|e| InstanceVO::from_instance(&e))
                .collect::<Vec<_>>(),
            dom: Some(key.service_name.to_owned()),
            ..Default::default()
        };
        serde_json::to_string(&result).unwrap()
    }

    pub fn get_ref_instance_list_string(
        clusters: String,
        key: &ServiceKey,
        v: Vec<&Arc<Instance>>,
    ) -> String {
        let now = Local::now().timestamp_millis();
        let result = QueryListResult {
            name: key.get_join_service_name(),
            cache_millis: 10000u64,
            last_ref_time: Some(now - 1000),
            checksum: Some(now.to_string()),
            use_specified_url: Some(false),
            clusters,
            env: Some("".to_owned()),
            hosts: v
                .into_iter()
                .map(|e| InstanceVO::from_instance(e))
                .collect::<Vec<_>>(),
            dom: Some(key.service_name.to_owned()),
            ..Default::default()
        };
        serde_json::to_string(&result).unwrap()
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InstanceVO {
    pub service: Arc<String>,
    pub ip: Arc<String>,
    pub port: u32,
    pub cluster_name: String,
    pub weight: f32,
    pub healthy: bool,
    pub instance_id: Arc<String>,
    pub metadata: Arc<HashMap<String, String>>,
    pub marked: Option<bool>,
    pub enabled: Option<bool>,
    pub service_name: Option<Arc<String>>,
    pub ephemeral: Option<bool>,
}

impl InstanceVO {
    pub fn from_instance(instance: &Instance) -> Self {
        Self {
            service: instance.group_service.clone(),
            ip: instance.ip.clone(),
            port: instance.port,
            cluster_name: instance.cluster_name.to_owned(),
            weight: instance.weight,
            healthy: instance.healthy,
            instance_id: instance.id.clone(),
            metadata: instance.metadata.clone(),
            marked: Some(true),
            enabled: Some(instance.enabled),
            //service_name: Some(instance.service_name.clone()),
            service_name: Some(instance.group_service.clone()),
            ephemeral: Some(instance.ephemeral),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ServiceQueryOptListRequest {
    pub page_no: Option<usize>,
    pub page_size: Option<usize>,
    pub namespace_id: Option<String>,
    pub group_name: Option<String>,
    pub service_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ServiceInfoParam {
    pub namespace_id: Option<String>,
    pub group_name: Option<String>,
    pub service_name: Option<String>,
    pub protect_threshold: Option<f32>,
    pub metadata: Option<String>,
    pub selector: Option<String>,
}

impl ServiceInfoParam {
    pub(crate) fn merge(self, b: Self) -> Self {
        Self {
            namespace_id: OptionUtils::select(self.namespace_id, b.namespace_id),
            group_name: OptionUtils::select(self.group_name, b.group_name),
            service_name: OptionUtils::select(self.service_name, b.service_name),
            protect_threshold: OptionUtils::select(self.protect_threshold, b.protect_threshold),
            metadata: OptionUtils::select(self.metadata, b.metadata),
            selector: OptionUtils::select(self.selector, b.selector),
        }
    }

    pub(crate) fn build_service_info(self) -> anyhow::Result<ServiceDetailDto> {
        if let Some(service_name) = self.service_name {
            if service_name.is_empty() {
                return Err(anyhow::anyhow!("service_name is valid"));
            }
            let metadata = if let Some(metadata_str) = self.metadata {
                match NamingUtils::parse_metadata(&metadata_str) {
                    Ok(metadata) => Some(Arc::new(metadata)),
                    Err(_) => None,
                }
            } else {
                None
            };

            Ok(ServiceDetailDto {
                namespace_id: Arc::new(NamingUtils::default_namespace(
                    self.namespace_id.unwrap_or_default(),
                )),
                service_name: Arc::new(service_name),
                group_name: Arc::new(NamingUtils::default_group(
                    self.group_name.unwrap_or_default(),
                )),
                metadata,
                protect_threshold: self.protect_threshold,
                ..Default::default()
            })
        } else {
            Err(anyhow::anyhow!("service_name is empty"))
        }
    }
}
