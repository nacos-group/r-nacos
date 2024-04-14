use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::naming::{
    model::{Instance, ServiceKey},
    NamingUtils,
};
use crate::utils::get_bool_from_string;

/*
#[derive(Debug,Serialize,Deserialize,Default)]
#[serde(rename_all = "camelCase")]
pub struct ServiceKeyParam{
    pub namespace_id:Option<String>,
    pub group_name:Option<String>,
    pub service_name:Option<String>,
}
*/

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct QueryAllInstanceListParam {
    pub namespace_id: Option<String>,
    pub group_name: Option<String>,
    pub service_name: Option<String>,
}

impl QueryAllInstanceListParam {
    pub fn to_service_key(self) -> anyhow::Result<ServiceKey> {
        if let Some(service_name) = self.service_name {
            if service_name.is_empty() {
                return Err(anyhow::anyhow!("param error,service is empty"));
            }
            let namespace_id = Arc::new(NamingUtils::default_namespace(
                self.namespace_id.unwrap_or_default(),
            ));
            let group_name = Arc::new(NamingUtils::default_group(
                self.group_name.unwrap_or_default(),
            ));
            Ok(ServiceKey::new_by_arc(
                namespace_id,
                group_name,
                Arc::new(service_name),
            ))
        } else {
            Err(anyhow::anyhow!("param error,service is empty"))
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpsNamingQueryListResponse {
    pub count: u64,
    pub list: Vec<Arc<Instance>>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ServiceParam {
    pub service_name: Arc<String>,
    pub namespace_id: Option<Arc<String>>,
    pub group_name: Option<Arc<String>>,
    pub metadata: Option<Arc<HashMap<String, String>>>,
    pub protect_threshold: Option<f32>,
}

impl ServiceParam {
    pub fn to_key(&self) -> ServiceKey {
        let group_name = self
            .group_name
            .clone()
            .unwrap_or(Arc::new("DEFAULT_GROUP".to_owned()));
        let namespace_id = self.namespace_id.clone().unwrap_or_default();
        ServiceKey::new_by_arc(namespace_id, group_name, self.service_name.clone())
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ServiceInfo {
    pub namespace_id: Arc<String>,
    pub service_name: Arc<String>,
    pub group_name: Arc<String>,
    pub metadata: Option<Arc<HashMap<String, String>>>,
    pub protect_threshold: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceParams {
    pub ip: Option<String>,
    pub port: Option<u32>,
    pub weight: Option<f32>,
    pub enabled: Option<String>,
    pub healthy: Option<String>,
    pub ephemeral: Option<String>,
    pub metadata: Option<String>,
    pub cluster_name: Option<String>,
    pub namespace_id: Option<String>,
    pub service_name: Arc<String>,
    pub group_name: Option<Arc<String>>,
}

impl InstanceParams {
    pub fn to_instance(self) -> Result<Instance, String> {
        let group_name = if let Some(v) = self.group_name {
            if v.is_empty() {
                Arc::new("DEFAULT_GROUP".to_string())
            } else {
                v
            }
        } else {
            Arc::new("DEFAULT_GROUP".to_string())
        };
        let mut instance = Instance {
            ip: Arc::new(self.ip.unwrap()),
            port: self.port.unwrap(),
            weight: self.weight.unwrap_or(1f32),
            enabled: get_bool_from_string(&self.enabled, true),
            healthy: true,
            ephemeral: get_bool_from_string(&self.ephemeral, true),
            cluster_name: NamingUtils::default_cluster(
                self.cluster_name
                    .as_ref()
                    .unwrap_or(&"".to_owned())
                    .to_owned(),
            ),
            namespace_id: Arc::new(NamingUtils::default_namespace(
                self.namespace_id
                    .as_ref()
                    .unwrap_or(&"".to_owned())
                    .to_owned(),
            )),
            service_name: self.service_name,
            group_name,
            ..Default::default()
        };

        let metadata_str = self
            .metadata
            .as_ref()
            .unwrap_or(&"{}".to_owned())
            .to_owned();
        if let Ok(metadata) = serde_json::from_str::<HashMap<String, String>>(&metadata_str) {
            instance.metadata = Arc::new(metadata);
        };
        instance.generate_key();
        Ok(instance)
    }
}
