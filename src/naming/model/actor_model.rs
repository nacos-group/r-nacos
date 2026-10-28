use crate::naming::model::{Instance, InstanceKey};
use crate::raft::filestore::model::SnapshotRecordDto;
use crate::raft::filestore::raftsnapshot::SnapshotWriterActor;
use actix::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// 命名服务Raft请求类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NamingRaftReq {
    /// 注册永久实例
    RegisterInstance { param: InstanceRegisterParam },
    /// 更新永久实例
    UpdateInstance { param: InstanceRegisterParam },
    /// 删除永久实例
    RemoveInstance(InstanceKey),
}

impl Message for NamingRaftReq {
    type Result = anyhow::Result<NamingRaftResult>;
}

/// 命名服务Raft响应类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NamingRaftResult {
    InstanceInfo(Arc<Instance>),
    None,
}

/// 实例注册参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceRegisterParam {
    pub ip: Arc<String>,
    pub port: u32,
    pub weight: f32,
    pub enabled: bool,
    pub healthy: bool,
    pub ephemeral: bool,
    pub metadata: Arc<HashMap<String, String>>,
    pub namespace_id: Arc<String>,
    pub group_name: Arc<String>,
    pub service_name: Arc<String>,
    pub cluster_name: Option<String>,
    pub app_name: Option<String>,
    pub last_modified_millis: i64,
}

impl From<InstanceRegisterParam> for Instance {
    fn from(param: InstanceRegisterParam) -> Self {
        let mut instance = Instance {
            ip: param.ip,
            port: param.port,
            namespace_id: param.namespace_id,
            group_name: param.group_name,
            service_name: param.service_name,
            weight: param.weight,
            enabled: param.enabled,
            healthy: param.healthy,
            ephemeral: param.ephemeral,
            metadata: param.metadata,
            ..Default::default()
        };
        if let Some(cluster_name) = param.cluster_name {
            instance.cluster_name = cluster_name;
        }
        if let Some(app_name) = param.app_name {
            instance.app_name = app_name;
        }
        instance.last_modified_millis = param.last_modified_millis;
        instance.generate_key();
        instance
    }
}

impl From<&Instance> for InstanceRegisterParam {
    fn from(instance: &Instance) -> Self {
        Self {
            ip: instance.ip.clone(),
            port: instance.port,
            weight: instance.weight,
            enabled: instance.enabled,
            healthy: instance.healthy,
            ephemeral: instance.ephemeral,
            metadata: instance.metadata.clone(),
            namespace_id: instance.namespace_id.clone(),
            group_name: instance.group_name.clone(),
            service_name: instance.service_name.clone(),
            cluster_name: Some(instance.cluster_name.clone()),
            app_name: Some(instance.app_name.clone()),
            last_modified_millis: instance.last_modified_millis,
        }
    }
}

/// 快照构建请求
#[derive(Debug, Clone, Message)]
#[rtype(result = "anyhow::Result<()>")]
pub struct SnapshotBuildRequest {
    pub writer: Addr<SnapshotWriterActor>,
}

/// 快照加载请求
#[derive(Debug, Clone, Message)]
#[rtype(result = "anyhow::Result<LoadResult>")]
pub struct SnapshotLoadRequest {
    pub record: SnapshotRecordDto,
}

/// 加载结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoadResult {
    Success,
    Error(String),
}

impl Default for InstanceRegisterParam {
    fn default() -> Self {
        Self {
            ip: Default::default(),
            port: Default::default(),
            weight: 1f32,
            enabled: true,
            healthy: true,
            ephemeral: true,
            metadata: Default::default(),
            namespace_id: Default::default(),
            group_name: Default::default(),
            service_name: Default::default(),
            //cluster_name: Some("DEFAULT".to_string()),
            cluster_name: None,
            app_name: None,
            last_modified_millis: 0,
        }
    }
}
