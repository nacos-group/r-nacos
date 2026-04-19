use std::collections::HashMap;
use std::sync::Arc;

use crate::common::pb::service_meta;
use crate::naming::model::{InstanceShortKey, ServiceKey};

#[derive(Debug, Clone)]
pub struct InstanceMetaDto {
    pub service_key: ServiceKey,
    pub instance_key: InstanceShortKey,
    pub metadata: Arc<HashMap<String, String>>,
}

impl InstanceMetaDto {
    pub fn new(
        service_key: ServiceKey,
        instance_key: InstanceShortKey,
        metadata: Arc<HashMap<String, String>>,
    ) -> Self {
        Self {
            service_key,
            instance_key,
            metadata,
        }
    }

    pub fn to_proto<'a>(&'a self) -> service_meta::InstanceMetaDo<'a> {
        let metadata: HashMap<_, _> = self
            .metadata
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        service_meta::InstanceMetaDo {
            namespace_id: self.service_key.namespace_id.as_str().into(),
            group_name: self.service_key.group_name.as_str().into(),
            service_name: self.service_key.service_name.as_str().into(),
            ip: self.instance_key.ip.as_str().into(),
            port: self.instance_key.port,
            metadata: metadata
                .into_iter()
                .map(|(k, v)| (std::borrow::Cow::Borrowed(k), std::borrow::Cow::Borrowed(v)))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct InstanceMetaDoOwned {
    pub namespace_id: String,
    pub group_name: String,
    pub service_name: String,
    pub ip: String,
    pub port: u32,
    pub metadata: HashMap<String, String>,
}

impl From<service_meta::InstanceMetaDo<'_>> for InstanceMetaDoOwned {
    fn from(value: service_meta::InstanceMetaDo) -> Self {
        Self {
            namespace_id: value.namespace_id.to_string(),
            group_name: value.group_name.to_string(),
            service_name: value.service_name.to_string(),
            ip: value.ip.to_string(),
            port: value.port,
            metadata: value
                .metadata
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }
}

impl From<InstanceMetaDoOwned> for InstanceMetaDto {
    fn from(value: InstanceMetaDoOwned) -> Self {
        Self {
            service_key: ServiceKey::new(
                &value.namespace_id,
                &value.group_name,
                &value.service_name,
            ),
            instance_key: InstanceShortKey {
                ip: Arc::new(value.ip),
                port: value.port,
            },
            metadata: Arc::new(value.metadata),
        }
    }
}
