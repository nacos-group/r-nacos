pub mod api;
pub mod cluster_api;
pub mod config_api;
pub mod connection_api;
pub mod login_api;
pub mod model;
pub mod naming_api;
pub mod user_api;

pub mod middle;
pub mod transfer_api;
pub mod v2;

use std::sync::Arc;

use self::model::NamespaceInfo;
use crate::namespace::model::{
    NamespaceFromFlags, NamespaceQueryReq, NamespaceQueryResult, NamespaceRaftReq,
};
use crate::{
    common::appdata::AppShareData,
    config::core::{ConfigActor, ConfigCmd, ConfigKey, ConfigResult},
    raft::cluster::model::SetConfigReq,
};
use actix::prelude::*;

pub struct NamespaceUtilsOld;

pub const DEFAULT_NAMESPACE: &str = "public";
pub const SYSCONFIG_NAMESPACE: &str = "__INNER_SYSTEM__";
pub const SYSCONFIG_GROUP: &str = "sys";
pub const SYSCONFIG_NAMESPACE_KEY: &str = "namespaces";

lazy_static::lazy_static! {
    static ref DEFAULT_NAMESPACE_INFO:Arc<NamespaceInfo> = Arc::new(NamespaceInfo {
            namespace_id: Some(Arc::new("".to_owned())),
            namespace_name: Some(DEFAULT_NAMESPACE.to_owned()),
            r#type: Some("0".to_owned()),
    });
}

impl NamespaceUtilsOld {
    pub async fn get_namespace_source(config_addr: &Addr<ConfigActor>) -> Arc<String> {
        let cmd = ConfigCmd::GET(ConfigKey::new(
            SYSCONFIG_NAMESPACE_KEY,
            SYSCONFIG_GROUP,
            SYSCONFIG_NAMESPACE,
        ));
        match config_addr.send(cmd).await {
            Ok(res) => {
                let r: ConfigResult = res.unwrap();
                match r {
                    ConfigResult::Data { value: v, .. } => v,
                    _ => Arc::new("".to_string()),
                }
            }
            Err(_) => Arc::new("".to_string()),
        }
    }
    pub fn get_namespaces_from_source(namespace_str: Arc<String>) -> Vec<Arc<NamespaceInfo>> {
        if namespace_str.is_empty() {
            return vec![DEFAULT_NAMESPACE_INFO.clone()];
        }
        if let Ok(mut namespaces) = serde_json::from_str::<Vec<Arc<NamespaceInfo>>>(&namespace_str)
        {
            let mut list = Vec::with_capacity(namespaces.len() + 1);
            list.push(DEFAULT_NAMESPACE_INFO.clone());
            list.append(&mut namespaces);
            list
        } else {
            vec![DEFAULT_NAMESPACE_INFO.clone()]
        }
    }

    pub async fn get_namespaces(config_addr: &Addr<ConfigActor>) -> Vec<Arc<NamespaceInfo>> {
        let namespace_str = Self::get_namespace_source(config_addr).await;
        Self::get_namespaces_from_source(namespace_str)
    }

    pub async fn load_namespace_from_config(config_addr: &Addr<ConfigActor>) -> Vec<NamespaceInfo> {
        let namespace_str = Self::get_namespace_source(config_addr).await;
        if namespace_str.is_empty() {
            return vec![];
        }
        serde_json::from_str::<Vec<NamespaceInfo>>(&namespace_str).unwrap_or_default()
    }

    pub async fn save_namespace(
        app_data: &Arc<AppShareData>,
        value: &Vec<NamespaceInfo>,
    ) -> anyhow::Result<()> {
        let value_str = serde_json::to_string(value)?;
        let req = SetConfigReq::new(
            ConfigKey::new(
                SYSCONFIG_NAMESPACE_KEY,
                SYSCONFIG_GROUP,
                SYSCONFIG_NAMESPACE,
            ),
            Arc::new(value_str),
        );
        match app_data.config_route.set_config(req).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!(err)),
        }
    }

    pub async fn add_namespace(
        app_data: &Arc<AppShareData>,
        info: NamespaceInfo,
    ) -> anyhow::Result<()> {
        if let (Some(namespace_id), Some(namespace_name)) = (info.namespace_id, info.namespace_name)
        {
            if namespace_id.is_empty() || namespace_id.as_ref().eq(DEFAULT_NAMESPACE) {
                return Err(anyhow::anyhow!("namespace is exist"));
            }
            let mut infos = Self::load_namespace_from_config(&app_data.config_addr).await;
            for item in &infos {
                if namespace_id.eq(item.namespace_id.as_ref().unwrap()) {
                    return Err(anyhow::anyhow!("namespace is exist"));
                }
            }
            let new_info = NamespaceInfo {
                namespace_id: Some(namespace_id),
                namespace_name: Some(namespace_name),
                r#type: Some("2".to_owned()),
            };
            infos.push(new_info);
            Self::save_namespace(app_data, &infos).await
        } else {
            Err(anyhow::anyhow!("params is empty"))
        }
    }

    pub async fn update_namespace(
        app_data: &Arc<AppShareData>,
        info: NamespaceInfo,
    ) -> anyhow::Result<()> {
        if let (Some(namespace_id), Some(namespace_name)) = (info.namespace_id, info.namespace_name)
        {
            if namespace_id.is_empty() || namespace_id.as_ref().eq(DEFAULT_NAMESPACE) {
                return Err(anyhow::anyhow!("namespace can't update"));
            }
            let infos = Self::load_namespace_from_config(&app_data.config_addr).await;
            let mut new_infos = Vec::with_capacity(infos.len());
            let mut update_mark = false;
            for mut item in infos {
                if namespace_id.eq(item.namespace_id.as_ref().unwrap()) {
                    item.namespace_name = Some(namespace_name.clone());
                    update_mark = true;
                }
                new_infos.push(item);
            }
            if !update_mark {
                return Err(anyhow::anyhow!("namespace is not exist"));
            }
            Self::save_namespace(app_data, &new_infos).await
        } else {
            Err(anyhow::anyhow!("params is empty"))
        }
    }

    pub async fn remove_namespace(
        app_data: &Arc<AppShareData>,
        namespace_id: Option<String>,
    ) -> anyhow::Result<()> {
        if let Some(namespace_id) = namespace_id {
            if namespace_id.is_empty() || namespace_id.eq(DEFAULT_NAMESPACE) {
                return Err(anyhow::anyhow!("namespace can't delete"));
            }
            let infos = Self::load_namespace_from_config(&app_data.config_addr).await;
            let infos_len = infos.len();
            let mut new_infos = Vec::with_capacity(infos.len());
            for item in infos {
                if namespace_id.eq(item.namespace_id.as_ref().unwrap() as &str) {
                    continue;
                }
                new_infos.push(item);
            }
            if new_infos.len() == infos_len {
                return Err(anyhow::anyhow!("namespace is not exist"));
            }
            Self::save_namespace(app_data, &new_infos).await
        } else {
            Err(anyhow::anyhow!("params is empty"))
        }
    }
}

pub struct NamespaceUtils;
impl NamespaceUtils {
    pub async fn get_namespaces(
        app_share_data: &Arc<AppShareData>,
    ) -> anyhow::Result<Vec<NamespaceInfo>> {
        let res = app_share_data
            .namespace_addr
            .send(NamespaceQueryReq::List)
            .await??;
        if let NamespaceQueryResult::List(list) = res {
            Ok(list
                .into_iter()
                .map(|e| e.as_ref().to_owned().into())
                .collect())
        } else {
            Err(anyhow::anyhow!("NamespaceQueryResult is error"))
        }
    }

    pub async fn add_namespace(
        app_data: &Arc<AppShareData>,
        info: NamespaceInfo,
    ) -> anyhow::Result<()> {
        app_data
            .raft_request_route
            .request_namespace(NamespaceRaftReq::Set(info.into()))
            .await?;
        Ok(())
    }

    pub async fn update_namespace(
        app_data: &Arc<AppShareData>,
        info: NamespaceInfo,
    ) -> anyhow::Result<()> {
        app_data
            .raft_request_route
            .request_namespace(NamespaceRaftReq::Update(info.into()))
            .await?;
        Ok(())
    }

    pub async fn remove_namespace(
        app_data: &Arc<AppShareData>,
        namespace_id: Option<Arc<String>>,
    ) -> anyhow::Result<()> {
        let namespace_id = namespace_id.unwrap_or_default();
        let res = app_data
            .namespace_addr
            .send(NamespaceQueryReq::Info(namespace_id.clone()))
            .await??;
        if let NamespaceQueryResult::Info(v) = res {
            if v.flag != NamespaceFromFlags::USER.bits() {
                // 命名空间下存在配置或服务数据，不能删除
                return Err(anyhow::anyhow!("The namespace can't be deleted,because config or service data exists in the namespace"));
            }
        }
        app_data
            .raft_request_route
            .request_namespace(NamespaceRaftReq::Delete { id: namespace_id })
            .await?;
        Ok(())
    }
}
