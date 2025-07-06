use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::{collections::HashMap, sync::Arc};

use crate::now_millis_i64;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    pub id: Arc<String>,
    pub ip: Arc<String>,
    pub port: u32,
    pub weight: f32,
    pub enabled: bool,
    pub healthy: bool,
    pub ephemeral: bool,
    pub cluster_name: String,
    pub service_name: Arc<String>,
    pub group_name: Arc<String>,
    pub group_service: Arc<String>,
    pub metadata: Arc<HashMap<String, String>>,
    pub last_modified_millis: i64,
    pub register_time: i64,
    pub namespace_id: Arc<String>,
    pub app_name: String,
    pub from_grpc: bool,
    //本节点管理的实例设置为0
    pub from_cluster: u64,
    pub client_id: Arc<String>,
}

impl Instance {
    pub fn new(ip: String, port: u32) -> Self {
        Self {
            ip: Arc::new(ip),
            port,
            ..Default::default()
        }
    }

    pub fn is_from_cluster(&self) -> bool {
        self.from_cluster > 0
    }

    pub fn is_enable_timeout(&self) -> bool {
        //grpc 不走过期检查
        !self.from_grpc && !self.is_from_cluster()
    }

    pub fn generate_key(&mut self) {
        //self.id = format!("{}#{}#{}#{}#{}",&self.ip,&self.port,&self.cluster_name,&self.service_name,&self.group_name)
        self.id = Arc::new(format!("{}#{}", &self.ip, &self.port))
    }

    pub fn init(&mut self) {
        self.last_modified_millis = now_millis_i64();
        if self.id.is_empty() {
            self.generate_key();
        }
    }

    pub fn check_valid(&self) -> bool {
        if self.id.is_empty()
            && self.port == 0
            && self.service_name.is_empty()
            && self.cluster_name.is_empty()
        {
            return false;
        }
        true
    }

    pub fn update_info(&self, o: &Self, _tag: Option<InstanceUpdateTag>) -> bool {
        self.enabled != o.enabled
            || self.healthy != o.healthy
            || self.weight != o.weight
            || self.ephemeral != o.ephemeral
            || self.metadata != o.metadata
    }

    pub fn get_service_key(&self) -> ServiceKey {
        //ServiceKey::new(&self.namespace_id,&self.group_name,&self.service_name)
        ServiceKey::new_by_arc(
            self.namespace_id.clone(),
            self.group_name.clone(),
            self.service_name.clone(),
        )
    }

    pub fn get_short_key(&self) -> InstanceShortKey {
        InstanceShortKey {
            ip: self.ip.clone(),
            port: self.port.to_owned(),
        }
    }

    pub fn get_instance_key(&self) -> InstanceKey {
        InstanceKey {
            namespace_id: self.namespace_id.clone(),
            group_name: self.group_name.clone(),
            service_name: self.service_name.clone(),
            ip: self.ip.clone(),
            port: self.port.to_owned(),
        }
    }

    pub fn get_id_string(&self) -> String {
        format!("{}#{}", &self.ip, &self.port)
    }
}

impl Default for Instance {
    fn default() -> Self {
        Self {
            id: Default::default(),
            ip: Default::default(),
            port: Default::default(),
            weight: 1f32,
            enabled: true,
            healthy: true,
            ephemeral: true,
            cluster_name: "DEFAULT".to_owned(),
            service_name: Default::default(),
            group_name: Default::default(),
            group_service: Default::default(),
            metadata: Default::default(),
            last_modified_millis: Default::default(),
            register_time: now_millis_i64(),
            namespace_id: Default::default(),
            app_name: Default::default(),
            from_grpc: false,
            from_cluster: 0,
            client_id: Default::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ServiceInfo {
    pub name: Option<Arc<String>>,
    pub group_name: Option<Arc<String>>,
    pub clusters: Option<String>,
    pub cache_millis: i64,
    pub hosts: Option<Vec<Arc<Instance>>>,
    pub last_ref_time: i64,
    pub checksum: i64,
    #[serde(rename = "allIPs")]
    pub all_ips: bool,
    pub reach_protection_threshold: bool,
    //pub metadata:Option<HashMap<String,String>>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ServiceDetailDto {
    pub namespace_id: Arc<String>,
    pub service_name: Arc<String>,
    pub group_name: Arc<String>,
    pub metadata: Option<Arc<HashMap<String, String>>>,
    pub protect_threshold: Option<f32>,
    pub grpc_instance_count: Option<i32>,
}

impl ServiceDetailDto {
    pub(crate) fn to_service_key(&self) -> ServiceKey {
        ServiceKey::new_by_arc(
            self.namespace_id.clone(),
            self.group_name.clone(),
            self.service_name.clone(),
        )
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstanceUpdateTag {
    pub weight: bool,
    pub metadata: bool,
    pub enabled: bool,
    pub ephemeral: bool,
    pub from_update: bool,
}

impl InstanceUpdateTag {
    pub fn is_al(&self) -> bool {
        self.weight && self.metadata && self.enabled && self.ephemeral
    }
    pub fn is_none(&self) -> bool {
        !self.weight && !self.metadata && !self.enabled && !self.ephemeral
    }
}

impl Default for InstanceUpdateTag {
    fn default() -> Self {
        Self {
            weight: true,
            metadata: true,
            enabled: true,
            ephemeral: true,
            from_update: false,
        }
    }
}

#[derive(Debug, Clone, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ServiceKey {
    pub namespace_id: Arc<String>,
    pub group_name: Arc<String>,
    pub service_name: Arc<String>,
}

impl ServiceKey {
    pub fn new(namespace_id: &str, group_name: &str, service_name: &str) -> Self {
        Self {
            namespace_id: Arc::new(namespace_id.to_owned()),
            group_name: Arc::new(group_name.to_owned()),
            service_name: Arc::new(service_name.to_owned()),
        }
    }

    pub fn new_by_arc(
        namespace_id: Arc<String>,
        group_name: Arc<String>,
        service_name: Arc<String>,
    ) -> Self {
        Self {
            namespace_id,
            group_name,
            service_name,
        }
    }

    pub fn get_join_service_name(&self) -> String {
        format!("{}@@{}", self.group_name, self.service_name)
    }
}

#[derive(Debug, Clone, Default, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceKey {
    pub namespace_id: Arc<String>,
    pub group_name: Arc<String>,
    pub service_name: Arc<String>,
    pub ip: Arc<String>,
    pub port: u32,
}

impl InstanceKey {
    pub fn new_by_service_key(key: &ServiceKey, ip: Arc<String>, port: u32) -> Self {
        Self {
            namespace_id: key.namespace_id.clone(),
            group_name: key.group_name.clone(),
            service_name: key.service_name.clone(),
            ip,
            port,
        }
    }

    pub fn get_service_key(&self) -> ServiceKey {
        ServiceKey::new_by_arc(
            self.namespace_id.clone(),
            self.group_name.clone(),
            self.service_name.clone(),
        )
    }

    pub fn get_short_key(&self) -> InstanceShortKey {
        InstanceShortKey {
            ip: self.ip.clone(),
            port: self.port.to_owned(),
        }
    }
}

#[derive(Debug, Clone, Default, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceShortKey {
    pub ip: Arc<String>,
    pub port: u32,
}

impl InstanceShortKey {
    pub fn new(ip: Arc<String>, port: u32) -> Self {
        Self { ip, port }
    }

    pub fn new_from_instance_id(id: &str) -> Self {
        let items: Vec<&str> = id.split('#').collect();
        assert!(items.len() > 1);
        let ip_str = items[0];
        let port_str = items[1];
        let port: u32 = port_str.parse().unwrap_or_default();
        Self {
            ip: Arc::new(ip_str.to_owned()),
            port,
        }
    }
}

#[derive(Debug, Clone)]
pub enum UpdateInstanceType {
    None,
    New,
    Remove,
    UpdateTime,
    UpdateValue,
    ///更新其它节点元信息
    UpdateOtherClusterMetaData(u64, Instance),
}

#[derive(Debug, Clone)]
pub enum DistroData {
    ClientInstances(HashMap<Arc<String>, HashSet<InstanceKey>>),
    #[deprecated]
    ServiceInstanceCount(HashMap<ServiceKey, u64>),
    DiffClientInstances(Vec<InstanceKey>),
}
