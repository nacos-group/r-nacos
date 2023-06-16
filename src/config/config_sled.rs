use std::convert::TryInto;

use super::{
    config::{ConfigKey, ConfigValue},
    dal::ConfigHistoryParam,
};
use crate::common::AppSysConfig;
use chrono::Local;
use prost::Message;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Message, Deserialize, Serialize)]
pub struct Config {
    #[prost(int64, optional, tag = "1")]
    pub id: Option<i64>,
    #[prost(string, tag = "2")]
    pub tenant: String,
    #[prost(string, tag = "3")]
    pub group: String,
    #[prost(string, tag = "4")]
    pub data_id: String,
    #[prost(string, optional, tag = "5")]
    pub content: Option<String>,
    #[prost(string, optional, tag = "6")]
    pub content_md5: Option<String>,
    #[prost(int64, optional, tag = "7")]
    pub last_time: Option<i64>,
}

impl Config {
    pub fn get_config_key(&self) -> String {
        format!("{}_{}_{}", self.tenant, self.group, self.data_id)
    }

    pub fn get_config_history_key(&self) -> String {
        let id = self.id.unwrap_or(0);
        format!("{}_{}_{}_{}", self.tenant, self.group, self.data_id, id)
    }
}

pub struct ConfigDB {
    // config data
    config_tree: sled::Tree,
    // config history data
    config_history_tree: sled::Tree,
    // config history id tracker
    config_history_helper: sled::Tree,
}

impl ConfigDB {
    pub fn new() -> Self {
        let sys_config = AppSysConfig::init_from_env();
        let db = sled::open(sys_config.config_db_dir).unwrap();
        let config_tree = db.open_tree("config").unwrap();
        let config_history_tree = db.open_tree("confighistory").unwrap();
        let config_history_helper = db.open_tree("confighistoryhelper").unwrap();

        Self {
            config_tree,
            config_history_tree,
            config_history_helper,
        }
    }

    pub fn update_config(&self, key: &ConfigKey, val: &ConfigValue) -> anyhow::Result<()> {
        let mut config = Config {
            id: None,
            tenant: key.tenant.as_ref().to_owned(),
            group: key.group.as_ref().to_owned(),
            data_id: key.data_id.as_ref().to_owned(),
            content: Some(val.content.as_ref().to_owned()),
            content_md5: Option::None,
            last_time: Some(Local::now().timestamp_millis()),
        };
        let config_key = config.get_config_key();

        if let Ok(Some(config_bytes)) = self.config_tree.get(&config_key) {
            // check if has any latest history
            if let Ok(Some(latest_id_bytes)) = self.config_history_helper.get(&config_key) {
                let latest_id = i64::from_ne_bytes(latest_id_bytes.as_ref().try_into()?);
                config.id = Some(latest_id + 1);
                self.config_history_helper
                    .insert(&config_key, &(latest_id + 1).to_ne_bytes())?;
            } else {
                self.config_history_helper
                    .insert(&config_key, &0i64.to_ne_bytes())?;
            }
            let his_key = config.get_config_history_key();
            self.config_history_tree.insert(his_key, config_bytes)?;
        }
        // using protobuf as value serialization
        let mut config_bytes = Vec::new();
        config.encode(&mut config_bytes)?;
        self.config_tree.insert(&config_key, config_bytes)?;
        Ok(())
    }

    pub fn del_config(&self, key: &ConfigKey) -> anyhow::Result<()> {
        let cfg = Config {
            tenant: key.tenant.as_ref().to_owned(),
            group: key.group.as_ref().to_owned(),
            data_id: key.data_id.as_ref().to_owned(),
            ..Default::default()
        };
        let config_key = cfg.get_config_key();

        if let Ok(Some(_)) = self.config_tree.remove(&config_key) {
            self.config_history_helper.remove(&config_key)?;
            let mut iter = self.config_history_tree.scan_prefix(&config_key);
            while let Some(Ok((k, _))) = iter.next() {
                self.config_history_tree.remove(k)?;
            }
        }

        Ok(())
    }

    pub fn query_config_list(&self) -> anyhow::Result<Vec<Config>> {
        let mut ret = vec![];
        let mut iter = self.config_tree.iter();
        while let Some(Ok((_, v))) = iter.next() {
            let cfg = Config::decode(v.as_ref())?;
            ret.push(cfg);
        }
        Ok(ret)
    }

    // total, current list
    pub fn query_config_history_page(
        &self,
        param: &ConfigHistoryParam,
    ) -> anyhow::Result<(usize, Vec<Config>)> {
        if let (Some(t), Some(g), Some(id)) = (&param.tenant, &param.group, &param.data_id) {
            let config_key = format!("{}_{}_{}", t, g, id);

            let iter = self.config_history_tree.scan_prefix(&config_key);
            let total = self.config_history_tree.scan_prefix(&config_key).count();

            let mut ret = vec![];
            if let Some(offset) = param.offset {
                let mut n_i = iter.skip(offset as usize);
                if let Some(limit) = param.limit {
                    let mut t = n_i.take(limit as usize);
                    while let Some(Ok((_, v))) = t.next() {
                        ret.push(Config::decode(v.as_ref())?);
                    }
                } else {
                    while let Some(Ok((_, v))) = n_i.next() {
                        ret.push(Config::decode(v.as_ref())?);
                    }
                }
            }
            return Ok((total, ret));
        }
        Ok((0, vec![]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test() {
        let config_db = ConfigDB::new();
        let key = ConfigKey {
            tenant: Arc::new("dev".to_owned()),
            group: Arc::new("dev".to_owned()),
            data_id: Arc::new("iris-app-dev.properties".to_owned()),
        };
        let val = ConfigValue {
            content: Arc::new(
                "appid=iris-app\r\nusername=iris\r\npass=1***5\r\nabc=123".to_owned(),
            ),
            md5: Arc::new("".to_owned()),
        };
        config_db.update_config(&key, &val).unwrap();

        let v = config_db.query_config_list().unwrap();
        println!("{:?}", v)
    }

    #[test]
    fn page_test() {
        let config_db = ConfigDB::new();
        let param = ConfigHistoryParam {
            id: None,
            tenant: Some("dev".to_owned()),
            group: Some("dev".to_owned()),
            data_id: Some("iris-app-dev.properties".to_owned()),
            order_by: None,
            order_by_desc: None,
            limit: Some(10),
            offset: Some(0),
        };
        // let mut iter = config_db.config_history_db.iter();
        // while let Some(Ok((k, v))) = iter.next() {
        //     let cfg = serde_json::from_slice::<Config>(&v).unwrap();
        //     println!("{:?} -> {:?}", String::from_utf8(k.to_vec()), cfg);
        // }
        let v = config_db.query_config_history_page(&param).unwrap();
        println!("{:?}, {:?}", v.0, v.1);
    }

    #[test]
    fn test_del() {
        let config_db = ConfigDB::new();
        let key = ConfigKey {
            tenant: Arc::new("dev".to_owned()),
            group: Arc::new("dev".to_owned()),
            data_id: Arc::new("iris-app-dev.properties".to_owned()),
        };
        config_db.del_config(&key).unwrap();
    }
}
