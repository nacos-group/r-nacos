use actix::prelude::*;
use serde::{Deserialize, Serialize};
use std::{convert::TryFrom, sync::Arc};

use crate::naming::model::{Instance, InstanceUpdateTag, ServiceDetailDto};

#[derive(Clone, Debug)]
pub enum NamingRouteAddr {
    Local(u64),
    Remote(u64, Arc<String>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdateInstanceReq {
    instance: Instance,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NamingRouteRequest {
    Ping(u64),
    UpdateInstance {
        instance: Instance,
        tag: Option<InstanceUpdateTag>,
    },
    RemoveInstance {
        instance: Instance,
    },
    SyncUpdateInstance {
        instance: Instance,
    },
    SyncRemoveInstance {
        instance: Instance,
    },
    QuerySnapshot {
        index: usize,
        len: usize,
    },
    Snapshot(Vec<u8>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NamingRouterResponse {
    None,
}

#[derive(Message, Debug, Clone)]
#[rtype(result = "anyhow::Result<SyncSenderResponse>")]
pub struct SyncSenderRequest(pub NamingRouteRequest);

#[derive(Message, Debug)]
#[rtype(result = "anyhow::Result<SyncSenderResponse>")]
pub enum SyncSenderSetCmd {
    UpdateTargetAddr(Arc<String>),
}

pub enum SyncSenderResponse {
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProcessRange {
    pub index: usize,
    pub len: usize,
}

impl ProcessRange {
    pub fn new(index: usize, len: usize) -> Self {
        Self { index, len }
    }

    pub fn is_range(&self, hash_value: usize) -> bool {
        self.len < 2 || (hash_value % self.len) == self.index
    }

    pub fn is_range_at_list(hash_value: usize, ranges: &Vec<Self>) -> bool {
        for range in ranges {
            if range.is_range(hash_value) {
                return true;
            }
        }
        false
    }
}

#[derive(Clone, PartialEq, prost::Message, Deserialize, Serialize)]
pub struct SnapshotDataInfo {
    #[prost(uint32, tag = "1")]
    route_index: u32,
    #[prost(uint32, tag = "2")]
    node_count: u32,
    /// ServiceDetailDto json
    #[prost(message, repeated, tag = "3")]
    pub services: Vec<String>,
    /// Instance json
    #[prost(message, repeated, tag = "4")]
    pub instances: Vec<String>,
}

impl SnapshotDataInfo {
    pub fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        use prost::Message;
        let mut data_bytes: Vec<u8> = Vec::new();
        self.encode(&mut data_bytes)?;
        Ok(data_bytes)
    }

    pub fn from_bytes(buf: &[u8]) -> anyhow::Result<Self> {
        use prost::Message;
        let s = SnapshotDataInfo::decode(buf)?;
        Ok(s)
    }
}

impl From<SnapshotForSend> for SnapshotDataInfo {
    fn from(v: SnapshotForSend) -> Self {
        Self {
            route_index: v.route_index as u32,
            node_count: v.node_count as u32,
            services: v
                .services
                .iter()
                .map(|e| serde_json::to_string(e).unwrap())
                .collect(),
            instances: v
                .instances
                .iter()
                .map(|e| serde_json::to_string(e).unwrap())
                .collect(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SnapshotForSend {
    pub route_index: u64,
    pub node_count: u64,
    pub services: Vec<ServiceDetailDto>,
    pub instances: Vec<Arc<Instance>>,
}

#[derive(Clone, Debug)]
pub struct SnapshotForReceive {
    pub route_index: u64,
    pub node_count: u64,
    pub services: Vec<ServiceDetailDto>,
    pub instances: Vec<Instance>,
}

impl TryFrom<SnapshotDataInfo> for SnapshotForReceive {
    type Error = anyhow::Error;

    fn try_from(value: SnapshotDataInfo) -> Result<Self, Self::Error> {
        let mut services = Vec::with_capacity(value.services.len());
        for e in &value.services {
            let v = serde_json::from_str(e)?;
            services.push(v);
        }
        let mut instances = Vec::with_capacity(value.instances.len());
        for e in &value.instances {
            let v = serde_json::from_str(e)?;
            instances.push(v);
        }
        Ok(Self {
            route_index: value.route_index as u64,
            node_count: value.node_count as u64,
            services,
            instances,
        })
    }
}
