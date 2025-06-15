use actix_http::HttpMessage;
use actix_web::HttpRequest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::naming::service::ServiceInfoDto;
use crate::naming::service_index::ServiceQueryParam;
use crate::naming::{
    model::{Instance, ServiceKey},
    NamingUtils,
};
use crate::user_namespace_privilege;
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

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ServiceQueryListRequest {
    pub page_no: Option<usize>,
    pub page_size: Option<usize>,
    pub namespace_id: Option<String>,
    pub group_name_param: Option<String>,
    pub service_name_param: Option<String>,
    /// 节点id,在查询指定节点信息时用到
    pub node_id: Option<u64>,
}

impl ServiceQueryListRequest {
    pub fn to_param(self, req: &HttpRequest) -> anyhow::Result<ServiceQueryParam> {
        let limit = self.page_size.unwrap_or(0xffff_ffff);
        let offset = (self.page_no.unwrap_or(1) - 1) * limit;
        let namespace_privilege = user_namespace_privilege!(req);
        let mut param = ServiceQueryParam {
            limit,
            offset,
            namespace_privilege,
            ..Default::default()
        };
        if let Some(namespace_id) = self.namespace_id {
            param.namespace_id = Some(Arc::new(NamingUtils::default_namespace(namespace_id)));
        }
        if let Some(group_name_param) = self.group_name_param {
            if !group_name_param.is_empty() {
                param.like_group = Some(group_name_param);
            }
        }
        if let Some(service_name_param) = self.service_name_param {
            if !service_name_param.is_empty() {
                param.like_service = Some(service_name_param);
            }
        }
        Ok(param)
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ServiceDto {
    pub name: Option<Arc<String>>,
    pub group_name: Option<Arc<String>>,
    pub cluster_count: Option<u64>,
    pub ip_count: Option<u64>,
    pub healthy_instance_count: Option<u64>,
    pub trigger_flag: Option<bool>,
    pub metadata: Option<String>,
    pub protect_threshold: Option<f32>,
}

impl From<ServiceInfoDto> for ServiceDto {
    fn from(value: ServiceInfoDto) -> Self {
        let metadata = value
            .metadata
            .as_ref()
            .map(|e| serde_json::to_string(e).unwrap_or_default());
        Self {
            name: Some(value.service_name),
            group_name: Some(value.group_name),
            cluster_count: Some(value.cluster_count as u64),
            ip_count: Some(value.instance_size as u64),
            healthy_instance_count: Some(value.healthy_instance_size as u64),
            trigger_flag: Some(value.trigger_flag),
            metadata,
            protect_threshold: value.protect_threshold,
        }
    }
}

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
    pub namespace_id: Option<String>,
    pub group_name: Option<String>,
    pub metadata: Option<String>,
    pub protect_threshold: Option<f32>,
}

impl ServiceParam {
    pub fn to_key(&self) -> ServiceKey {
        let group_name = Arc::new(NamingUtils::default_group(
            self.group_name.clone().unwrap_or_default(),
        ));
        let namespace_id = Arc::new(NamingUtils::default_namespace(
            self.namespace_id.clone().unwrap_or_default(),
        ));
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
    pub enabled: Option<bool>,
    pub healthy: Option<String>,
    pub ephemeral: Option<String>,
    pub metadata: Option<String>,
    pub cluster_name: Option<String>,
    pub namespace_id: Option<String>,
    pub service_name: Arc<String>,
    pub group_name: Option<String>,
}

impl InstanceParams {
    pub fn to_instance(self) -> Result<Instance, String> {
        let group_name = Arc::new(NamingUtils::default_group(
            self.group_name.clone().unwrap_or_default(),
        ));
        let namespace_id = Arc::new(NamingUtils::default_namespace(
            self.namespace_id.clone().unwrap_or_default(),
        ));
        let mut instance = Instance {
            ip: Arc::new(self.ip.unwrap()),
            port: self.port.unwrap(),
            weight: self.weight.unwrap_or(1f32),
            enabled: self.enabled.unwrap_or(true),
            healthy: true,
            ephemeral: get_bool_from_string(&self.ephemeral, true),
            cluster_name: NamingUtils::default_cluster(
                self.cluster_name
                    .as_ref()
                    .unwrap_or(&"".to_owned())
                    .to_owned(),
            ),
            service_name: self.service_name,
            group_name,
            namespace_id,
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
