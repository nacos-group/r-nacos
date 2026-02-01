#![allow(unused_imports)]

use actix_http::HttpMessage;
use actix_web::HttpRequest;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

use crate::naming::api_model::InstanceVO;
use crate::naming::model::ServiceKey;
use crate::naming::{service::ServiceInfoDto, service_index::ServiceQueryParam, NamingUtils};
use crate::user_namespace_privilege;

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpsServiceQueryListRequest {
    pub page_no: Option<usize>,
    pub page_size: Option<usize>,
    pub namespace_id: Option<String>,
    pub group_name_param: Option<String>,
    pub service_name_param: Option<String>,
}

impl OpsServiceQueryListRequest {
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
pub struct OpsServiceDto {
    pub name: Option<Arc<String>>,
    pub group_name: Option<Arc<String>>,
    pub cluster_count: Option<u64>,
    pub ip_count: Option<u64>,
    pub healthy_instance_count: Option<u64>,
    pub trigger_flag: Option<bool>,
    pub metadata: Option<String>,
    pub protect_threshold: Option<f32>,
}

impl From<ServiceInfoDto> for OpsServiceDto {
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
pub struct OpsServiceOptQueryListResponse {
    pub count: u64,
    pub service_list: Vec<OpsServiceDto>,
}

impl OpsServiceOptQueryListResponse {
    pub fn new(count: u64, service_list: Vec<OpsServiceDto>) -> Self {
        Self {
            count,
            service_list,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpsQueryServiceInstanceListRequest {
    pub namespace_id: Option<String>,
    pub service_name: Option<String>,
    pub group_name: Option<String>,
    pub cluster_name: Option<String>,
    pub page_no: Option<usize>,
    pub page_size: Option<usize>,
}

impl OpsQueryServiceInstanceListRequest {
    pub(crate) fn to_clusters_key(&self) -> Result<(ServiceKey, String), String> {
        let grouped_name = self.service_name.clone().unwrap_or_default();
        let (mut group_name, service_name) =
            NamingUtils::split_group_and_service_name(&grouped_name)
                .ok_or_else(|| "serviceName is invalid!".to_owned())?;
        if let Some(_group_name) = self.group_name.as_ref() {
            if !_group_name.is_empty() {
                _group_name.clone_into(&mut group_name)
            }
        }
        let namespace_id = NamingUtils::default_namespace(
            self.namespace_id
                .as_ref()
                .unwrap_or(&"".to_owned())
                .to_owned(),
        );
        let key = ServiceKey::new(&namespace_id, &group_name, &service_name);

        Ok((
            key,
            self.cluster_name
                .as_ref()
                .unwrap_or(&"".to_owned())
                .to_owned(),
        ))
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpsQueryServiceInstanceListResponse {
    pub count: u64,
    pub list: Vec<InstanceVO>,
}

impl OpsQueryServiceInstanceListResponse {
    pub fn new(count: u64, list: Vec<InstanceVO>) -> Self {
        Self { count, list }
    }
}
