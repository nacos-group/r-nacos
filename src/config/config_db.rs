use std::rc::Rc;
use chrono::Local;
use rusqlite::Connection;
use crate::common::AppSysConfig;

use super::{config::{ConfigKey, ConfigValue}, dal::{ConfigDO, ConfigDao, ConfigHistoryDO, ConfigHistoryDao, ConfigParam,ConfigHistoryParam}};

pub struct ConfigDB {
    config_dao:ConfigDao,
    config_history_dao:ConfigHistoryDao,
}

impl Default for ConfigDB {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigDB {
    pub fn new() -> Self {
        let sys_config = AppSysConfig::init_from_env();
        let config_db = sys_config.config_db_file;
        let conn = Connection::open(config_db).unwrap();
        Self::init(&conn);
        let conn = Rc::new(conn);
        let config_dao = ConfigDao::new(conn.clone());
        let config_history_dao = ConfigHistoryDao::new(conn);
        Self{
            config_dao,
            config_history_dao,
        }
    }

    fn init(conn:&Connection){
        let create_table_sql = r"
create table if not exists tb_config(
    id integer primary key autoincrement,
    data_id varchar(255),
    `group` varchar(255),
    tenant varchar(255),
    content text,
    content_md5 varchar(36),
    last_time long
);
create index if not exists tb_config_key_idx on tb_config(data_id,`group`,tenant);

create table if not exists tb_config_history(
    id integer primary key autoincrement,
    data_id varchar(255),
    `group` varchar(255),
    tenant varchar(255),
    content text,
    last_time long
);
create index if not exists tb_config_history_key_idx on tb_config_history(data_id,`group`,tenant);
        ";
        conn.execute_batch(create_table_sql).unwrap();
    }

    fn convert_to_config_do(key:&ConfigKey,val:&ConfigValue) -> ConfigDO {
        let current_time = Local::now().timestamp_millis();
        ConfigDO {
            tenant : Some(key.tenant.as_ref().to_owned()),
            group : Some(key.group.as_ref().to_owned()),
            data_id: Some(key.data_id.as_ref().to_owned()),
            content: Some(val.content.as_ref().to_owned()),
            content_md5: Some(val.md5.as_ref().to_owned()),
            last_time : Some(current_time),
            ..Default::default()
        }
    }

    fn convert_to_config_history_do(key:&ConfigKey,val:&ConfigValue) -> ConfigHistoryDO {
        let current_time = Local::now().timestamp_millis();
        ConfigHistoryDO {
            tenant : Some(key.tenant.as_ref().to_owned()),
            group : Some(key.group.as_ref().to_owned()),
            data_id: Some(key.data_id.as_ref().to_owned()),
            content: Some(val.content.as_ref().to_owned()),
            last_time : Some(current_time),
            ..Default::default()
        }
    }

    fn convert_to_config_param(key:&ConfigKey) -> ConfigParam {
        ConfigParam {
            tenant : Some(key.tenant.as_ref().to_owned()),
            group : Some(key.group.as_ref().to_owned()),
            data_id: Some(key.data_id.as_ref().to_owned()),
            ..Default::default()
        }
    }

    fn convert_to_config_history_param(key:&ConfigKey) -> ConfigHistoryParam {
        ConfigHistoryParam {
            tenant : Some(key.tenant.as_ref().to_owned()),
            group : Some(key.group.as_ref().to_owned()),
            data_id: Some(key.data_id.as_ref().to_owned()),
            ..Default::default()
        }
    }

    pub fn update_config(&self,key:&ConfigKey,val:&ConfigValue) {
        let config = Self::convert_to_config_do(key, val);
        let config_history = Self::convert_to_config_history_do(key, val);
        let config_param = Self::convert_to_config_param(key);
        let is_update=match self.config_dao.update(&config,&config_param) {
            Ok(size) => {
                size >0
            },
            _ => {false}
        };
        if ! is_update {
            self.config_dao.insert(&config).unwrap();
        }
        self.config_history_dao.insert(&config_history).unwrap();
    }

    pub fn del_config(&self,key:&ConfigKey){
        let config_param = Self::convert_to_config_param(key);
        let config_history_param = Self::convert_to_config_history_param(key);
        self.config_dao.delete(&config_param).unwrap();
        self.config_history_dao.delete(&config_history_param).unwrap();
    }

    pub fn query_config_list(&self) -> Vec<ConfigDO> {
        let param = ConfigParam::default();
        self.config_dao.query(&param).unwrap()
    }

    pub fn query_config_history_page(&self,param:&ConfigHistoryParam) -> (usize,Vec<ConfigHistoryDO>) {
        let size = self.config_history_dao.query_count(param).unwrap_or_default();
        let list = self.config_history_dao.query(param).unwrap_or_default();
        (size as usize,list)
    }

}