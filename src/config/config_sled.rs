use std::collections::HashMap;

use crate::common;

use super::{
    core::{ConfigKey, ConfigValue},
    dal::ConfigHistoryParam,
};

use anyhow::Result;
use chrono::Local;
use prost::Message;
use serde::{Deserialize, Serialize};

const CONFIG_HISTORY_TREE_NAME_PREFIX: &str = "config_history_";

#[derive(Clone, PartialEq, Message, Deserialize, Serialize)]
pub struct Config {
    #[prost(string, tag = "1")]
    pub tenant: String,
    #[prost(string, tag = "2")]
    pub group: String,
    #[prost(string, tag = "3")]
    pub data_id: String,
    #[prost(string, optional, tag = "4")]
    pub content: Option<String>,
    #[prost(string, optional, tag = "5")]
    pub content_md5: Option<String>,
    #[prost(int64, optional, tag = "6")]
    pub last_time: Option<i64>,
    #[prost(int64, optional, tag = "7")]
    pub id: Option<i64>,
}

impl Config {
    fn get_config_key(&self) -> Result<Vec<u8>> {
        let csk = ConfigSerdeKey {
            tenant: self.tenant.to_owned(),
            group: self.group.to_owned(),
            data_id: self.data_id.to_owned(),
        };
        Ok(csk.to_key())
    }

    fn get_config_history_tree_name(&self) -> Vec<u8> {
        Self::build_config_history_tree_name(&mut self.get_config_key().unwrap())
    }

    fn get_config_history_key(&self) -> Result<Vec<u8>> {
        // FIXME: 如何保证key的顺序
        //let vec = format!("{:0>4}", self.id.unwrap_or(1)).as_bytes().to_vec();
        let vec = self.id.unwrap_or(1).to_be_bytes().to_vec();
        Ok(vec)
    }

    fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut v = Vec::new();
        self.encode(&mut v)?;
        Ok(v)
    }

    fn build_config_history_tree_name(config_key: &mut Vec<u8>) -> Vec<u8> {
        let mut tree_name = CONFIG_HISTORY_TREE_NAME_PREFIX.as_bytes().to_vec();
        tree_name.append(config_key);
        tree_name
    }
}

/**
 * Config Key Protobuf
 */
#[derive(Message)]
struct ConfigSerdeKey {
    #[prost(string, tag = "1")]
    pub tenant: String,
    #[prost(string, tag = "2")]
    pub group: String,
    #[prost(string, tag = "3")]
    pub data_id: String,
}

impl ConfigSerdeKey {
    fn to_key(&self) -> Vec<u8> {
        let mut v = Vec::new();
        self.encode(&mut v).unwrap();
        v
    }
}

pub struct ConfigDB {
    // config data
    config_db: sled::Tree,
    // config history data
    config_history: HashMap<Vec<u8>, sled::Tree>,
    config_history_ids: HashMap<Vec<u8>, i64>,
}

impl Default for ConfigDB {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigDB {
    pub fn new() -> Self {
        let db = common::DB.lock().unwrap();
        let config_db = db.open_tree("config").unwrap();

        // init all config_histories & config_history_ids
        let mut config_history = HashMap::new();
        let mut config_history_ids = HashMap::new();
        let mut iter = config_db.iter();

        while let Some(Ok((k, _))) = iter.next() {
            let mut k_bytes = k.to_vec();
            let config_history_tree_name = Config::build_config_history_tree_name(&mut k_bytes);
            let cht = db.open_tree(&config_history_tree_name).unwrap();

            // try to get latest config history id
            if let Some(Ok((_, v))) = cht.iter().last() {
                let latest_cfg = Config::decode(v.as_ref()).unwrap();
                config_history_ids.insert(
                    config_history_tree_name.to_owned(),
                    latest_cfg.id.unwrap_or(0),
                );
            } else {
                config_history_ids.insert(config_history_tree_name.to_owned(), 0);
            }
            config_history.insert(config_history_tree_name, cht);
        }

        Self {
            config_db,
            config_history,
            config_history_ids,
        }
    }

    pub fn new_his_tree(&mut self, key: &Vec<u8>) -> Result<()> {
        let db = common::DB.lock().unwrap();
        let tree = db.open_tree(key)?;
        self.config_history.insert(key.to_owned(), tree);
        Ok(())
    }

    pub fn update_config(&mut self, key: &ConfigKey, val: &ConfigValue) -> Result<()> {
        let mut config = Config {
            tenant: key.tenant.as_ref().to_owned(),
            group: key.group.as_ref().to_owned(),
            data_id: key.data_id.as_ref().to_owned(),
            content: Some(val.content.as_ref().to_owned()),
            content_md5: Option::None,
            last_time: Some(Local::now().timestamp_millis()),
            id: None,
        };

        let config_key = config.get_config_key()?;
        let config_history_tree_name = config.get_config_history_tree_name();

        // using protobuf as value serialization
        self.config_db.insert(&config_key, config.to_bytes()?)?;

        // update config history id
        match self.config_history_ids.get(&config_key) {
            Some(id) => {
                config.id = Some(id + 1);
                self.config_history_ids
                    .insert(config_key.to_owned(), id + 1);
            }
            None => {
                config.id = Some(1);
                self.config_history_ids.insert(config_key.to_owned(), 1);
            }
        }

        let his_tree = match self.config_history.get(&config_history_tree_name) {
            Some(tree) => tree,
            None => {
                self.new_his_tree(&config_history_tree_name)?;
                self.config_history.get(&config_history_tree_name).unwrap()
            }
        };

        let his_key = config.get_config_history_key()?;
        his_tree.insert(his_key, config.to_bytes()?)?;

        Ok(())
    }

    pub fn del_config(&mut self, key: &ConfigKey) -> Result<()> {
        let cfg = Config {
            tenant: key.tenant.as_ref().to_owned(),
            group: key.group.as_ref().to_owned(),
            data_id: key.data_id.as_ref().to_owned(),
            ..Default::default()
        };

        let config_key = cfg.get_config_key()?;
        let config_history_tree_name = cfg.get_config_history_tree_name();
        if let Ok(Some(_)) = self.config_db.remove(&config_key) {
            self.config_history_ids.remove(&config_key);

            if self.config_history.get(&config_history_tree_name).is_some() {
                let db = common::DB.lock().unwrap();
                db.drop_tree(&config_history_tree_name)?;
                self.config_history.remove(&config_history_tree_name);
            }
        }

        Ok(())
    }

    pub fn query_config_list(&self) -> Result<Vec<Config>> {
        let mut ret = vec![];
        let mut iter = self.config_db.iter();
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
    ) -> Result<(usize, Vec<Config>)> {
        if let (Some(t), Some(g), Some(id)) = (&param.tenant, &param.group, &param.data_id) {
            let config = ConfigSerdeKey {
                tenant: t.to_owned(),
                group: g.to_owned(),
                data_id: id.to_owned(),
            };
            let mut config_key = config.to_key();
            let history_name = Config::build_config_history_tree_name(&mut config_key);

            if let Some(his_tree) = self.config_history.get(&history_name) {
                let total: usize = his_tree.len();
                let mut ret = vec![];
                let iter = his_tree.iter().rev();
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
        }
        Ok((0, vec![]))
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::get_md5;

    use super::*;
    use std::sync::Arc;

    #[test]
    fn test() {
        let mut config_db = ConfigDB::new();
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
    fn multi_insert() {
        let mut config_db = ConfigDB::new();
        for i in 1..1000 {
            let key = ConfigKey {
                tenant: Arc::new("dev".to_owned()),
                group: Arc::new("dev".to_owned()),
                data_id: Arc::new("iris-app-dev.properties".to_owned()),
            };
            let val_string = format!("appid=iris-app\r\nusername=iris\r\npass=1***5\r\nid={}", i);
            let val = ConfigValue {
                md5: Arc::new(get_md5(&val_string)),
                content: Arc::new(val_string),
            };
            config_db.update_config(&key, &val).unwrap();
        }
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
        let mut config_db = ConfigDB::new();
        let key = ConfigKey {
            tenant: Arc::new("dev".to_owned()),
            group: Arc::new("dev".to_owned()),
            data_id: Arc::new("iris-app-dev.properties".to_owned()),
        };
        config_db.del_config(&key).unwrap();
    }

    #[test]
    fn list() {
        let config_db = ConfigDB::new();
        let v = config_db.query_config_list().unwrap();
        println!("{:?}", v)
    }
}
