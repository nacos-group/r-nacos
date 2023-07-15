pub mod api;
pub mod config_api;
pub mod connection_api;
pub mod model;
pub mod naming_api;
mod raft_api;

use std::sync::Arc;

use crate::config::core::{ConfigActor, ConfigCmd, ConfigKey, ConfigResult};
use actix::prelude::*;

use self::model::NamespaceInfo;

pub struct NamespaceUtils;

pub const DEFAULT_NAMESPACE: &str = "public";
pub const SYSCONFIG_NAMESPACE: &str = "__INNER_SYSTEM__";
pub const SYSCONFIG_GROUP: &str = "sys";
pub const SYSCONFIG_NAMESPACE_KEY: &str = "namespaces";

lazy_static::lazy_static! {
    static ref DEFAULT_NAMESPACE_INFO:Arc<NamespaceInfo> = Arc::new(NamespaceInfo {
            namespace_id: Some("".to_owned()),
            namespace_name: Some(DEFAULT_NAMESPACE.to_owned()),
            r#type: Some("0".to_owned()),
    });
}

impl NamespaceUtils {
    pub async fn get_namespaces(config_addr: &Addr<ConfigActor>) -> Vec<Arc<NamespaceInfo>> {
        let cmd = ConfigCmd::GET(ConfigKey::new(
            SYSCONFIG_NAMESPACE_KEY,
            SYSCONFIG_GROUP,
            SYSCONFIG_NAMESPACE,
        ));
        let namespace_str = match config_addr.send(cmd).await {
            Ok(res) => {
                let r: ConfigResult = res.unwrap();
                match r {
                    ConfigResult::DATA(v, _md5) => v,
                    _ => Arc::new("".to_string()),
                }
            }
            Err(_) => Arc::new("".to_string()),
        };
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

    pub async fn load_namespace_from_config(config_addr: &Addr<ConfigActor>) -> Vec<NamespaceInfo> {
        let cmd = ConfigCmd::GET(ConfigKey::new(
            SYSCONFIG_NAMESPACE_KEY,
            SYSCONFIG_GROUP,
            SYSCONFIG_NAMESPACE,
        ));
        let namespace_str = match config_addr.send(cmd).await {
            Ok(res) => {
                let r: ConfigResult = res.unwrap();
                match r {
                    ConfigResult::DATA(v, _md5) => v,
                    _ => Arc::new("".to_string()),
                }
            }
            Err(_) => Arc::new("".to_string()),
        };
        if namespace_str.is_empty() {
            return vec![];
        }
        if let Ok(namespaces) = serde_json::from_str::<Vec<NamespaceInfo>>(&namespace_str) {
            namespaces
        } else {
            vec![]
        }
    }

    pub async fn save_namespace(
        config_addr: &Addr<ConfigActor>,
        value: &Vec<NamespaceInfo>,
    ) -> anyhow::Result<()> {
        let value_str = serde_json::to_string(value)?;
        let cmd = ConfigCmd::ADD(
            ConfigKey::new(
                SYSCONFIG_NAMESPACE_KEY,
                SYSCONFIG_GROUP,
                SYSCONFIG_NAMESPACE,
            ),
            Arc::new(value_str),
        );
        match config_addr.send(cmd).await {
            Ok(res) => {
                let _: ConfigResult = res.unwrap();
                Ok(())
            }
            Err(err) => Err(anyhow::anyhow!(err)),
        }
    }

    pub async fn add_namespace(
        config_addr: &Addr<ConfigActor>,
        info: NamespaceInfo,
    ) -> anyhow::Result<()> {
        if let (Some(namespace_id), Some(namespace_name)) = (info.namespace_id, info.namespace_name)
        {
            if namespace_id.is_empty() || namespace_id.eq(DEFAULT_NAMESPACE) {
                return Err(anyhow::anyhow!("namespace is exist"));
            }
            let mut infos = Self::load_namespace_from_config(config_addr).await;
            for item in &infos {
                if namespace_id.eq(item.namespace_id.as_ref().unwrap() as &str) {
                    return Err(anyhow::anyhow!("namespace is exist"));
                }
            }
            let new_info = NamespaceInfo {
                namespace_id: Some(namespace_id),
                namespace_name: Some(namespace_name),
                r#type: Some("2".to_owned()),
            };
            infos.push(new_info);
            Self::save_namespace(config_addr, &infos).await
        } else {
            Err(anyhow::anyhow!("params is empty"))
        }
    }

    pub async fn update_namespace(
        config_addr: &Addr<ConfigActor>,
        info: NamespaceInfo,
    ) -> anyhow::Result<()> {
        if let (Some(namespace_id), Some(namespace_name)) = (info.namespace_id, info.namespace_name)
        {
            if namespace_id.is_empty() || namespace_id.eq(DEFAULT_NAMESPACE) {
                return Err(anyhow::anyhow!("namespace can't update"));
            }
            let infos = Self::load_namespace_from_config(config_addr).await;
            let mut new_infos = Vec::with_capacity(infos.len());
            let mut update_mark = false;
            for mut item in infos {
                if namespace_id.eq(item.namespace_id.as_ref().unwrap() as &str) {
                    item.namespace_name = Some(namespace_name.clone());
                    update_mark = true;
                }
                new_infos.push(item);
            }
            if !update_mark {
                return Err(anyhow::anyhow!("namespace is not exist"));
            }
            Self::save_namespace(config_addr, &new_infos).await
        } else {
            Err(anyhow::anyhow!("params is empty"))
        }
    }

    pub async fn remove_namespace(
        config_addr: &Addr<ConfigActor>,
        namespace_id: Option<String>,
    ) -> anyhow::Result<()> {
        if let Some(namespace_id) = namespace_id {
            if namespace_id.is_empty() || namespace_id.eq(DEFAULT_NAMESPACE) {
                return Err(anyhow::anyhow!("namespace can't delete"));
            }
            let infos = Self::load_namespace_from_config(config_addr).await;
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
            Self::save_namespace(config_addr, &new_infos).await
        } else {
            Err(anyhow::anyhow!("params is empty"))
        }
    }
}
