use crate::config::config_index::ConfigQueryParam;
use crate::config::core::{ConfigInfoDto, ConfigKey};
use crate::config::dal::ConfigHistoryParam;
use crate::config::ConfigUtils;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpsConfigImportInfo {
    pub tenant: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpsConfigQueryListRequest {
    pub page_no: Option<usize>,
    pub page_size: Option<usize>,
    pub tenant: Option<String>,
    pub group_param: Option<String>,
    pub data_param: Option<String>,
    pub group: Option<String>,
    pub data_id: Option<String>,
}

impl OpsConfigQueryListRequest {
    pub fn to_param(self) -> anyhow::Result<ConfigQueryParam> {
        let limit = self.page_size.unwrap_or(0xffff_ffff);
        let offset = (self.page_no.unwrap_or(1) - 1) * limit;
        let mut param = ConfigQueryParam {
            limit,
            offset,
            like_group: self.group_param,
            like_data_id: self.data_param,
            ..Default::default()
        };
        if let Some(tenant) = self.tenant {
            param.tenant = Some(Arc::new(ConfigUtils::default_tenant(tenant)));
        } else {
            param.tenant = Some(Arc::new("".to_owned()));
        }
        Ok(param)
    }

    pub fn to_history_param(self) -> anyhow::Result<ConfigHistoryParam> {
        if let (Some(group), Some(data_id)) = (&self.group, &self.data_id) {
            if group.is_empty() || data_id.is_empty() {
                return Err(anyhow::anyhow!("group or dataId can't empty"));
            }
        } else {
            return Err(anyhow::anyhow!("group or dataId can't empty"));
        }
        let limit = self.page_size.unwrap_or(0xffff_ffff) as i64;
        let offset = (self.page_no.unwrap_or(1) - 1) as i64 * limit;
        let mut param = ConfigHistoryParam {
            limit: Some(limit),
            offset: Some(offset),
            data_id: self.data_id,
            group: self.group,
            order_by: Some("last_time".to_owned()),
            order_by_desc: Some(true),
            ..Default::default()
        };
        if let Some(tenant) = self.tenant {
            param.tenant = Some(ConfigUtils::default_tenant(tenant));
        } else {
            param.tenant = Some("".to_owned());
        }
        Ok(param)
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpsConfigOptQueryListResponse {
    pub count: u64,
    pub list: Vec<ConfigInfoDto>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConfigParams {
    pub data_id: Arc<String>,
    pub group: Option<Arc<String>>,
    pub tenant: Option<String>,
    pub content: Option<Arc<String>>,
    pub config_type: Option<Arc<String>>,
    pub desc: Option<Arc<String>>,
}

impl ConfigParams {
    pub fn to_key(&self) -> ConfigKey {
        let group = self
            .group
            .clone()
            .unwrap_or(Arc::new("DEFAULT_GROUP".to_owned()));
        let tenant = ConfigUtils::default_tenant(self.tenant.clone().unwrap_or_default());
        ConfigKey::new_by_arc(self.data_id.clone(), group, Arc::new(tenant))
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConfigInfo {
    pub value: Option<Arc<String>>,
    pub md5: Option<Arc<String>>,
    pub config_type: Option<Arc<String>>,
    pub desc: Option<Arc<String>>,
}
