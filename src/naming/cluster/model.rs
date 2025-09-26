use crate::metrics::timeline::model::{TimelineQueryParam, TimelineQueryResponse};
use crate::naming::model::{Instance, InstanceKey, InstanceUpdateTag, ServiceDetailDto};
use crate::naming::service::SubscriberInfoDto;
use crate::naming::service_index::ServiceQueryParam;
use crate::openapi::mcp::model::JsonRpcRequest;
use actix::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::{convert::TryFrom, sync::Arc};

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
    SyncUpdateService {
        service: ServiceDetailDto,
    },
    SyncBatchInstances(Vec<u8>),
    RemoveClientId {
        client_id: Arc<String>,
    },
    QuerySnapshot {
        index: usize,
        len: usize,
    },
    Snapshot(Vec<u8>),
    MetricsTimelineQuery(TimelineQueryParam),
    SyncDistroClientInstances(HashMap<Arc<String>, HashSet<InstanceKey>>),
    QueryDistroInstanceSnapshot(Vec<InstanceKey>),
    QueryServiceSubscriberPage(ServiceQueryParam),
    McpMessages {
        session_id: Arc<String>,
        server_key: Arc<String>,
        request: JsonRpcRequest,
        headers: HashMap<String, String>,
    },
}

impl NamingRouteRequest {
    pub fn get_sub_name(&self) -> &'static str {
        match self {
            NamingRouteRequest::Ping(_) => "Ping",
            NamingRouteRequest::UpdateInstance { .. } => "UpdateInstance",
            NamingRouteRequest::RemoveInstance { .. } => "RemoveInstance",
            NamingRouteRequest::SyncUpdateInstance { .. } => "SyncUpdateInstance",
            NamingRouteRequest::SyncRemoveInstance { .. } => "SyncRemoveInstance",
            NamingRouteRequest::SyncUpdateService { .. } => "SyncUpdateService",
            NamingRouteRequest::SyncBatchInstances(_) => "SyncBatchInstances",
            NamingRouteRequest::RemoveClientId { .. } => "RemoveClientId",
            NamingRouteRequest::QuerySnapshot { .. } => "QuerySnapshot",
            NamingRouteRequest::Snapshot(_) => "Snapshot",
            NamingRouteRequest::MetricsTimelineQuery(_) => "MetricsTimelineQuery",
            NamingRouteRequest::SyncDistroClientInstances(_) => "SyncDistroClientInstances",
            NamingRouteRequest::QueryDistroInstanceSnapshot(_) => "QueryDistroInstanceSnapshot",
            NamingRouteRequest::QueryServiceSubscriberPage(_) => "QueryServiceSubscriberPage",
            NamingRouteRequest::McpMessages { .. } => "McpMessages",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NamingRouterResponse {
    None,
    MetricsTimeLineResponse(TimelineQueryResponse),
    ServiceSubscribersPage((usize, Vec<SubscriberInfoDto>)),
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
pub struct SyncBatchDataInfo {
    #[prost(message, repeated, tag = "1")]
    pub update_instances: Vec<String>,
    #[prost(message, repeated, tag = "2")]
    pub remove_instances: Vec<String>,
}

impl SyncBatchDataInfo {
    pub fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        use prost::Message;
        let mut data_bytes: Vec<u8> = Vec::new();
        self.encode(&mut data_bytes)?;
        Ok(data_bytes)
    }

    pub fn from_bytes(buf: &[u8]) -> anyhow::Result<Self> {
        use prost::Message;
        let s = SyncBatchDataInfo::decode(buf)?;
        Ok(s)
    }
}

#[derive(Clone, Debug)]
pub struct SyncBatchForSend {
    pub update_instances: Vec<Arc<Instance>>,
    pub remove_instances: Vec<Arc<Instance>>,
}

#[derive(Clone, Debug)]
pub struct SyncBatchForReceive {
    pub update_instances: Vec<Instance>,
    pub remove_instances: Vec<Instance>,
}

impl From<SyncBatchForSend> for SyncBatchDataInfo {
    fn from(v: SyncBatchForSend) -> Self {
        Self {
            update_instances: v
                .update_instances
                .iter()
                .map(|e| serde_json::to_string(e).unwrap_or_default())
                .collect(),
            remove_instances: v
                .remove_instances
                .iter()
                .map(|e| serde_json::to_string(e).unwrap_or_default())
                .collect(),
        }
    }
}

impl TryFrom<SyncBatchDataInfo> for SyncBatchForReceive {
    type Error = anyhow::Error;

    fn try_from(value: SyncBatchDataInfo) -> Result<Self, Self::Error> {
        let mut update_instances = Vec::with_capacity(value.update_instances.len());
        for e in &value.update_instances {
            let v = serde_json::from_str(e)?;
            update_instances.push(v);
        }
        let mut remove_instances = Vec::with_capacity(value.remove_instances.len());
        for e in &value.remove_instances {
            let v = serde_json::from_str(e)?;
            remove_instances.push(v);
        }
        Ok(Self {
            update_instances,
            remove_instances,
        })
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
    #[prost(uint32, tag = "5")]
    pub mode: u32,
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
            mode: v.mode,
            services: v
                .services
                .iter()
                .map(|e| serde_json::to_string(e).unwrap_or_default())
                .collect(),
            instances: v
                .instances
                .iter()
                .map(|e| serde_json::to_string(e).unwrap_or_default())
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
    /// 0: 同步节点增量数据
    /// 1: 同步diff distor服务全量实例数据（不在里面的实例需要删除）
    pub mode: u32,
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
