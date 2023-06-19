use std::{collections::HashMap, sync::RwLock};

use crate::common;

use super::{
    config::{ConfigKey, ConfigValue},
    dal::ConfigHistoryParam,
};

use anyhow::Result;
use chrono::Local;
use prost::Message;
use serde::{Deserialize, Serialize};

const MAX_HIS_CNT: i64 = 9999;

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
    pub fn get_config_key(&self) -> Result<Vec<u8>> {
        let csk = ConfigSerdeKey {
            tenant: self.tenant.to_owned(),
            group: self.group.to_owned(),
            data_id: self.data_id.to_owned(),
        };
        let mut v = Vec::new();
        csk.encode(&mut v)?;
        Ok(v)
    }

    pub fn get_config_history_key(&self) -> Result<Vec<u8>> {
        let vec = format!("{:0>4}", self.id.unwrap_or(1)).as_bytes().to_vec();
        Ok(vec)
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

/**
 * Config History Id Protobuf
 */
#[derive(Message)]
struct ConfigHistoryId {
    #[prost(int64, tag = "1")]
    pub id: i64,
}

pub struct ConfigDB {
    // config data
    config_db: sled::Tree,
    // config history data
    config_history: RwLock<HashMap<Vec<u8>, sled::Tree>>,
    // config history id tracker [0001 ~ 9999]
    config_history_helper: sled::Tree,
}

impl Default for ConfigDB {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigDB {
    pub fn new() -> Self {
        let db = common::DB.lock().unwrap();
        let config_tree = db.open_tree("config").unwrap();
        let config_history_helper = db.open_tree("confighistoryhelper").unwrap();

        // init all config_histories
        let mut map = HashMap::new();
        let mut iter = config_tree.iter();
        while let Some(Ok((k, _))) = iter.next() {
            let k_bytes = k.to_vec();
            let cht = db.open_tree(&k_bytes).unwrap();
            map.insert(k_bytes, cht);
        }

        Self {
            config_db: config_tree,
            config_history: RwLock::new(map),
            config_history_helper,
        }
    }

    // pub fn new_his_tree(&mut self, key: &Vec<u8>) -> Result<()> {
    //     let mut ch = self.config_history.write().unwrap();
    //     let db = common::DB.lock().unwrap();
    //     let tree = db.open_tree(key)?;
    //     ch.insert(key.to_owned(), tree);
    //     Ok(())
    // }

    pub fn update_config(&mut self, key: &ConfigKey, val: &ConfigValue) -> Result<()> {
        let mut config = Config {
            id: Some(1),
            tenant: key.tenant.as_ref().to_owned(),
            group: key.group.as_ref().to_owned(),
            data_id: key.data_id.as_ref().to_owned(),
            content: Some(val.content.as_ref().to_owned()),
            content_md5: Option::None,
            last_time: Some(Local::now().timestamp_millis()),
        };

        let config_key = config.get_config_key()?;

        if let Ok(Some(config_bytes)) = self.config_db.get(&config_key) {
            let mut not_over_limit = true;
            // deal with history id counter
            if let Ok(Some(latest_id_bytes)) = self.config_history_helper.get(&config_key) {
                let mut latest_id = ConfigHistoryId::decode(latest_id_bytes.as_ref())?;
                if latest_id.id >= MAX_HIS_CNT {
                    not_over_limit = false;
                    log::warn!("config history id reach max limit: {}", MAX_HIS_CNT);
                }

                if not_over_limit {
                    config.id = Some(latest_id.id + 1);
                    latest_id.id += 1;
                    let mut data = Vec::new();
                    latest_id.encode(&mut data)?;
                    self.config_history_helper.insert(&config_key, data)?;
                }
            } else {
                let mut data = Vec::new();
                let latest_id = ConfigHistoryId { id: 1 };
                latest_id.encode(&mut data)?;
                self.config_history_helper.insert(&config_key, data)?;
            }

            let mut ch = self.config_history.write().unwrap();
            let his_tree = match ch.get(&config_key) {
                Some(tree) => tree,
                None => {
                    let db = common::DB.lock().unwrap();
                    let tree = db.open_tree(&config_key)?;
                    ch.insert(config_key.to_owned(), tree);
                    ch.get(&config_key).unwrap()
                }
            };

            if not_over_limit {
                // have to update id in old_config_bytes
                let mut cfg = Config::decode(config_bytes.as_ref())?;
                cfg.id = config.id;
                let mut data = Vec::new();
                cfg.encode(&mut data)?;
                let his_key = config.get_config_history_key()?;
                his_tree.insert(his_key, data)?;
            }
        }

        // using protobuf as value serialization
        let mut config_bytes = Vec::new();
        config.encode(&mut config_bytes)?;
        self.config_db.insert(&config_key, config_bytes)?;
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
        if let Ok(Some(_)) = self.config_db.remove(&config_key) {
            self.config_history_helper.remove(&config_key)?;
            let mut ch = self.config_history.write().unwrap();

            if ch.get(&config_key).is_some() {
                let db = common::DB.lock().unwrap();
                db.drop_tree(&config_key)?;
                ch.remove(&config_key);
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
            let config = Config {
                tenant: t.to_owned(),
                group: g.to_owned(),
                data_id: id.to_owned(),
                ..Default::default()
            };
            let config_key = config.get_config_key()?;
            let ch = self.config_history.read().unwrap();

            if let Some(his_tree) = ch.get(&config_key) {
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
                tenant: Arc::new("".to_owned()),
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
