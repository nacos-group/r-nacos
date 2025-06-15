#![allow(unused_imports, unused_assignments, unused_variables)]
use crate::common::option_utils::OptionUtils;
use crate::naming::model::{Instance, ServiceKey};
use crate::naming::service::SubscriberInfoDto;
use crate::naming::NamingUtils;
use crate::utils::get_bool_from_string;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceWebParams {
    pub ip: Option<String>,
    pub port: Option<u32>,
    pub namespace_id: Option<String>,
    pub weight: Option<f32>,
    pub enabled: Option<String>,
    pub healthy: Option<String>,
    pub ephemeral: Option<String>,
    pub metadata: Option<String>,
    pub cluster_name: Option<String>,
    pub service_name: Option<String>,
    pub group_name: Option<String>,
}

impl InstanceWebParams {
    pub(crate) fn merge(self, o: Self) -> Self {
        Self {
            ip: OptionUtils::select(self.ip, o.ip),
            port: OptionUtils::select(self.port, o.port),
            namespace_id: OptionUtils::select(self.namespace_id, o.namespace_id),
            weight: OptionUtils::select(self.weight, o.weight),
            enabled: OptionUtils::select(self.enabled, o.enabled),
            healthy: OptionUtils::select(self.healthy, o.healthy),
            ephemeral: OptionUtils::select(self.ephemeral, o.ephemeral),
            metadata: OptionUtils::select(self.metadata, o.metadata),
            cluster_name: OptionUtils::select(self.cluster_name, o.cluster_name),
            service_name: OptionUtils::select(self.service_name, o.service_name),
            group_name: OptionUtils::select(self.group_name, o.group_name),
        }
    }

    pub(crate) fn convert_to_instance(self) -> Result<Instance, String> {
        let ip = if let Some(ip) = self.ip {
            Arc::new(ip)
        } else {
            return Err("the instance ip is None".to_string());
        };
        let port = if let Some(port) = self.port {
            port
        } else {
            return Err("the instance port is None".to_string());
        };
        let mut instance = Instance {
            ip,
            port,
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
            ..Default::default()
        };

        let grouped_name = self.service_name.unwrap_or_default();
        if let Some((group_name, service_name)) =
            NamingUtils::split_group_and_service_name(&grouped_name)
        {
            instance.service_name = Arc::new(service_name);
            instance.group_name = Arc::new(group_name);
        } else {
            return Err("serviceName is invalid!".to_owned());
        }
        if let Some(group_name) = self.group_name {
            if !group_name.is_empty() {
                instance.group_name = Arc::new(group_name);
            }
        }
        let metadata_str = self
            .metadata
            .as_ref()
            .unwrap_or(&"{}".to_owned())
            .to_owned();
        if let Ok(metadata) = NamingUtils::parse_metadata(&metadata_str) {
            instance.metadata = Arc::new(metadata);
        };
        instance.generate_key();
        Ok(instance)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceWebQueryListParams {
    pub namespace_id: Option<String>,
    pub service_name: Option<String>,
    pub group_name: Option<String>,
    pub clusters: Option<String>,
    pub healthy_only: Option<String>,
    #[serde(rename = "clientIP")]
    pub client_ip: Option<String>,
    pub udp_port: Option<String>,
}

impl InstanceWebQueryListParams {
    pub(crate) fn to_clusters_key(&self) -> Result<(ServiceKey, String), String> {
        let mut service_name = "".to_owned();
        let mut group_name = "".to_owned();
        let grouped_name = self.service_name.clone().unwrap_or_default();
        if let Some((_group_name, _service_name)) =
            NamingUtils::split_group_and_service_name(&grouped_name)
        {
            service_name = _service_name;
            group_name = _group_name;
        } else {
            return Err("serviceName is invalid!".to_owned());
        }
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

        /*
        let mut clusters = vec![];
        if let Some(cluster_str) = self.clusters.as_ref() {
            clusters = cluster_str.split(",").into_iter()
                .filter(|e|{e.len()>0}).map(|e|{e.to_owned()}).collect::<Vec<_>>();
        }
        */
        Ok((
            key,
            self.clusters.as_ref().unwrap_or(&"".to_owned()).to_owned(),
        ))
    }

    pub(crate) fn get_addr(&self) -> Option<SocketAddr> {
        let port: Option<u16> = self
            .udp_port
            .as_ref()
            .map(|e| e.parse().unwrap_or_default());
        if let Some(port) = &port {
            if *port == 0u16 {
                return None;
            }
            if let Some(ip_str) = &self.client_ip {
                if let Ok(ip) = ip_str.parse() {
                    return Some(SocketAddr::new(ip, *port));
                }
            }
        }
        None
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BeatRequest {
    pub namespace_id: Option<String>,
    pub service_name: Option<String>,
    pub cluster_name: Option<String>,
    pub group_name: Option<String>,
    pub ephemeral: Option<String>,
    pub beat: Option<String>,
    pub ip: Option<String>,
    pub port: Option<u32>,
}

impl BeatRequest {
    pub(crate) fn merge(self, o: Self) -> Self {
        Self {
            namespace_id: OptionUtils::select(self.namespace_id, o.namespace_id),
            cluster_name: OptionUtils::select(self.cluster_name, o.cluster_name),
            service_name: OptionUtils::select(self.service_name, o.service_name),
            group_name: OptionUtils::select(self.group_name, o.group_name),
            ephemeral: OptionUtils::select(self.ephemeral, o.ephemeral),
            beat: OptionUtils::select(self.beat, o.beat),
            ip: OptionUtils::select(self.ip, o.ip),
            port: OptionUtils::select(self.port, o.port),
        }
    }

    /*
    pub fn convert_to_instance_old(self) -> Result<Instance, String> {
        let beat = match self.beat {
            Some(v) => v,
            None => {
                return Err("beat value is empty".to_string());
            }
        };
        let beat_info = match serde_json::from_str::<BeatInfo>(&beat) {
            Ok(v) => v,
            Err(err) => {
                return Err(err.to_string());
            }
        };
        let service_name_option = beat_info.service_name.clone();
        let mut instance = beat_info.convert_to_instance();
        if service_name_option.is_none() {
            let grouped_name = self.service_name.unwrap();
            if let Some((group_name, service_name)) =
                NamingUtils::split_group_and_service_name(&grouped_name)
            {
                instance.service_name = Arc::new(service_name);
                instance.group_name = Arc::new(group_name);
            }
            if let Some(group_name) = self.group_name.as_ref() {
                if !group_name.is_empty() {
                    instance.group_name = Arc::new(group_name.to_owned());
                }
            }
        }
        instance.ephemeral = get_bool_from_string(&self.ephemeral, true);
        instance.cluster_name = NamingUtils::default_cluster(
            self.cluster_name
                .as_ref()
                .unwrap_or(&"".to_owned())
                .to_owned(),
        );
        instance.namespace_id = Arc::new(NamingUtils::default_namespace(
            self.namespace_id
                .as_ref()
                .unwrap_or(&"".to_owned())
                .to_owned(),
        ));
        instance.generate_key();
        Ok(instance)
    }
    */

    pub fn convert_to_instance(self) -> anyhow::Result<Instance> {
        let mut beat_info = self.get_beat_info()?;
        let use_beat = self.beat.as_ref().is_some_and(|s| !s.is_empty());
        if !use_beat {
            beat_info.ip = self.ip;
            beat_info.port = self.port;
        }
        if beat_info.ip.is_none() || beat_info.port.is_none() {
            return Err(anyhow::anyhow!("ip or port is empty".to_owned()));
        }
        let service_name_option = beat_info.service_name.clone();
        let mut instance = beat_info.convert_to_instance();
        if service_name_option.is_none() {
            if let Some(grouped_name) = self.service_name {
                if let Some((group_name, service_name)) =
                    NamingUtils::split_group_and_service_name(&grouped_name)
                {
                    instance.service_name = Arc::new(service_name);
                    instance.group_name = Arc::new(group_name);
                }
            } else {
                return Err(anyhow::anyhow!("service name is empty".to_owned()));
            }
            if let Some(group_name) = self.group_name {
                if !group_name.is_empty() {
                    instance.group_name = Arc::new(group_name);
                }
            }
        }
        instance.ephemeral = get_bool_from_string(&self.ephemeral, true);
        instance.cluster_name = NamingUtils::default_cluster(self.cluster_name.unwrap_or_default());
        instance.namespace_id = Arc::new(NamingUtils::default_namespace(
            self.namespace_id.unwrap_or_default(),
        ));
        instance.generate_key();
        Ok(instance)
    }

    fn get_beat_info(&self) -> anyhow::Result<BeatInfo> {
        let beat_str = self.beat.clone().unwrap_or_default();
        if let Some(beat_str) = self.beat.as_ref() {
            let v = serde_json::from_str::<BeatInfo>(beat_str)?;
            Ok(v)
        } else {
            Ok(BeatInfo::default())
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BeatInfo {
    pub cluster: Option<String>,
    pub ip: Option<String>,
    pub port: Option<u32>,
    pub metadata: Option<HashMap<String, String>>,
    pub period: Option<i64>,
    pub scheduled: Option<bool>,
    pub service_name: Option<String>,
    pub stopped: Option<bool>,
    pub weight: Option<f32>,
}

impl BeatInfo {
    pub fn convert_to_instance(self) -> Instance {
        let mut instance = Instance {
            ip: Arc::new(self.ip.unwrap_or("unknown".to_string())),
            port: self.port.unwrap_or(1),
            cluster_name: NamingUtils::default_cluster(
                self.cluster.as_ref().unwrap_or(&"".to_owned()).to_owned(),
            ),
            ..Default::default()
        };
        if let Some(grouped_name) = self.service_name.as_ref() {
            if let Some((group_name, service_name)) =
                NamingUtils::split_group_and_service_name(grouped_name)
            {
                instance.service_name = Arc::new(service_name);
                instance.group_name = Arc::new(group_name);
            }
        }
        if let Some(metadata) = self.metadata {
            instance.metadata = Arc::new(metadata);
        }
        //instance.generate_key();
        instance
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ServiceQueryListRequest {
    pub page_no: Option<usize>,
    pub page_size: Option<usize>,
    pub namespace_id: Option<String>,
    pub group_name: Option<String>,
    pub service_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ServiceQueryListResponce {
    pub count: usize,
    pub doms: Vec<Arc<String>>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ServiceQuerySubscribersListResponce {
    pub count: usize,
    pub subscribers: Vec<SubscriberInfoDto>,
}
