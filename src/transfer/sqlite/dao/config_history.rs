#[allow(dead_code, unused_imports)]
use rsql_builder::B;
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::common::rusqlite_utils::{
    get_row_arc_value, get_row_value, sqlite_execute, sqlite_fetch, sqlite_fetch_count,
};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ConfigHistoryDO {
    pub id: Option<i64>,
    pub data_id: Option<Arc<String>>,
    pub group_id: Option<Arc<String>>,
    pub tenant_id: Option<Arc<String>>,
    pub content: Option<Arc<String>>,
    pub config_type: Option<String>,
    pub config_desc: Option<String>,
    pub op_user: Option<Arc<String>>,
    pub last_time: Option<i64>,
}

impl ConfigHistoryDO {
    fn from_row(r: &Row) -> Self {
        let mut s = Self::default();
        s.id = get_row_value(r, "id");
        s.data_id = get_row_arc_value(r, "data_id");
        s.group_id = get_row_arc_value(r, "group_id");
        s.tenant_id = get_row_arc_value(r, "tenant_id");
        s.content = get_row_arc_value(r, "content");
        s.config_type = get_row_value(r, "config_type");
        s.config_desc = get_row_value(r, "config_desc");
        s.op_user = get_row_arc_value(r, "op_user");
        s.last_time = get_row_value(r, "last_time");
        s
    }
}

#[derive(Debug, Default)]
pub struct ConfigHistoryParam {
    pub id: Option<i64>,
    pub data_id: Option<Arc<String>>,
    pub group_id: Option<Arc<String>>,
    pub tenant_id: Option<Arc<String>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
pub struct ConfigHistorySql {}

impl ConfigHistorySql {
    fn conditions(&self, param: &ConfigHistoryParam) -> B<'_> {
        let mut whr = B::new_where();
        if let Some(id) = &param.id {
            whr.eq("id", id);
        }
        if let Some(data_id) = &param.data_id {
            whr.eq("data_id", data_id);
        }
        if let Some(group_id) = &param.group_id {
            whr.eq("group_id", group_id);
        }
        if let Some(tenant_id) = &param.tenant_id {
            whr.eq("tenant_id", tenant_id);
        }
        whr
    }

    pub fn query_prepare(&self, param: &ConfigHistoryParam) -> (String, Vec<serde_json::Value>) {
        B::prepare(
            B::new_sql("select id, data_id, group_id, tenant_id, content, config_type, config_desc, op_user, last_time from tb_config_history")
                .push_build(&mut self.conditions(param))
        )
    }

    pub fn insert_prepare(&self, record: &ConfigHistoryDO) -> (String, Vec<serde_json::Value>) {
        let mut field_builder = B::new_comma_paren();
        let mut value_builder = B::new_comma_paren();
        if let Some(id) = &record.id {
            field_builder.push_sql("id");
            value_builder.push("?", id);
        }
        if let Some(data_id) = &record.data_id {
            field_builder.push_sql("data_id");
            value_builder.push("?", data_id);
        }
        if let Some(group_id) = &record.group_id {
            field_builder.push_sql("group_id");
            value_builder.push("?", group_id);
        }
        if let Some(tenant_id) = &record.tenant_id {
            field_builder.push_sql("tenant_id");
            value_builder.push("?", tenant_id);
        }
        if let Some(content) = &record.content {
            field_builder.push_sql("content");
            value_builder.push("?", content);
        }
        if let Some(config_type) = &record.config_type {
            field_builder.push_sql("config_type");
            value_builder.push("?", config_type);
        }
        if let Some(config_desc) = &record.config_desc {
            field_builder.push_sql("config_desc");
            value_builder.push("?", config_desc);
        }
        if let Some(op_user) = &record.op_user {
            field_builder.push_sql("op_user");
            value_builder.push("?", op_user);
        }
        if let Some(last_time) = &record.last_time {
            field_builder.push_sql("last_time");
            value_builder.push("?", last_time);
        }
        B::prepare(
            B::new_sql("insert into tb_config_history")
                .push_build(&mut field_builder)
                .push_sql("values")
                .push_build(&mut value_builder),
        )
    }

    pub fn update_prepare(
        &self,
        record: &ConfigHistoryDO,
        param: &ConfigHistoryParam,
    ) -> (String, Vec<serde_json::Value>) {
        let mut set_builder = B::new_comma();
        if let Some(id) = &record.id {
            set_builder.eq("id", id);
        }
        if let Some(data_id) = &record.data_id {
            set_builder.eq("data_id", data_id);
        }
        if let Some(group_id) = &record.group_id {
            set_builder.eq("group_id", group_id);
        }
        if let Some(tenant_id) = &record.tenant_id {
            set_builder.eq("tenant_id", tenant_id);
        }
        if let Some(content) = &record.content {
            set_builder.eq("content", content);
        }
        if let Some(config_type) = &record.config_type {
            set_builder.eq("config_type", config_type);
        }
        if let Some(config_desc) = &record.config_desc {
            set_builder.eq("config_desc", config_desc);
        }
        if let Some(op_user) = &record.op_user {
            set_builder.eq("op_user", op_user);
        }
        if let Some(last_time) = &record.last_time {
            set_builder.eq("last_time", last_time);
        }
        let mut whr = self.conditions(param);
        if whr.is_empty() {
            panic!("update conditions is empty");
        }
        B::prepare(
            B::new_sql("update tb_config_history set ")
                .push_build(&mut set_builder)
                .push_build(&mut whr),
        )
    }

    pub fn delete_prepare(&self, param: &ConfigHistoryParam) -> (String, Vec<serde_json::Value>) {
        B::prepare(
            B::new_sql("delete from tb_config_history").push_build(&mut self.conditions(param)),
        )
    }
}

pub struct ConfigHistoryDao<'a> {
    conn: &'a Connection,
    inner: ConfigHistorySql,
}

impl<'a> ConfigHistoryDao<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self {
            conn,
            inner: ConfigHistorySql {},
        }
    }

    pub fn execute(&self, sql: &str, args: &[serde_json::Value]) -> anyhow::Result<usize> {
        sqlite_execute(self.conn, sql, args)
    }

    pub fn fetch(
        &self,
        sql: &str,
        args: &[serde_json::Value],
    ) -> anyhow::Result<Vec<ConfigHistoryDO>> {
        sqlite_fetch(self.conn, sql, args, ConfigHistoryDO::from_row)
    }

    pub fn fetch_count(&self, sql: &str, args: &[serde_json::Value]) -> anyhow::Result<u64> {
        sqlite_fetch_count(self.conn, sql, args)
    }

    pub fn insert(&self, record: &ConfigHistoryDO) -> anyhow::Result<usize> {
        let (sql, args) = self.inner.insert_prepare(record);
        self.execute(&sql, &args)
    }

    pub fn update(
        &self,
        record: &ConfigHistoryDO,
        param: &ConfigHistoryParam,
    ) -> anyhow::Result<usize> {
        let (sql, args) = self.inner.update_prepare(record, param);
        self.execute(&sql, &args)
    }

    pub fn delete(&self, param: &ConfigHistoryParam) -> anyhow::Result<usize> {
        let (sql, args) = self.inner.delete_prepare(param);
        self.execute(&sql, &args)
    }

    pub fn query(&self, param: &ConfigHistoryParam) -> anyhow::Result<Vec<ConfigHistoryDO>> {
        let (sql, args) = self.inner.query_prepare(param);
        self.fetch(&sql, &args)
    }
}
