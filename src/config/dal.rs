use rsql_builder::{IBuilder, B};
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};
use std::rc::Rc;

use crate::common::rusqlite_utils::{
    get_row_value, sqlite_execute, sqlite_fetch, sqlite_fetch_count,
};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ConfigDO {
    pub id: Option<i64>,
    pub data_id: Option<String>,
    pub group: Option<String>,
    pub tenant: Option<String>,
    pub content: Option<String>,
    pub content_md5: Option<String>,
    pub last_time: Option<i64>,
}

impl ConfigDO {
    fn from_row(r: &Row) -> Self {
        Self {
            id: get_row_value(r, "id"),
            data_id: get_row_value(r, "data_id"),
            group: get_row_value(r, "group"),
            tenant: get_row_value(r, "tenant"),
            content: get_row_value(r, "content"),
            content_md5: get_row_value(r, "content_md5"),
            last_time: get_row_value(r, "last_time"),
        }
    }
}

#[derive(Debug, Default)]
pub struct ConfigParam {
    pub id: Option<i64>,
    pub data_id: Option<String>,
    pub group: Option<String>,
    pub tenant: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
pub struct ConfigSql {}

impl ConfigSql {
    fn conditions(&self, param: &ConfigParam) -> B<'_> {
        let mut whr = B::new_where();
        if let Some(id) = &param.id {
            whr.eq("id", id);
        }
        if let Some(data_id) = &param.data_id {
            whr.eq("data_id", data_id);
        }
        if let Some(group) = &param.group {
            whr.eq("`group`", group);
        }
        if let Some(tenant) = &param.tenant {
            whr.eq("tenant", tenant);
        }
        whr
    }

    pub fn query_prepare(&self, param: &ConfigParam) -> (String, Vec<serde_json::Value>) {
        B::prepare(
            B::new_sql("select id, data_id, `group`, tenant, content, content_md5, last_time from tb_config")
            .push_build(&mut self.conditions(param))
        )
    }

    pub fn insert_prepare(&self, record: &ConfigDO) -> (String, Vec<serde_json::Value>) {
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
        if let Some(group) = &record.group {
            field_builder.push_sql("`group`");
            value_builder.push("?", group);
        }
        if let Some(tenant) = &record.tenant {
            field_builder.push_sql("tenant");
            value_builder.push("?", tenant);
        }
        if let Some(content) = &record.content {
            field_builder.push_sql("content");
            value_builder.push("?", content);
        }
        if let Some(content_md5) = &record.content_md5 {
            field_builder.push_sql("content_md5");
            value_builder.push("?", content_md5);
        }
        if let Some(last_time) = &record.last_time {
            field_builder.push_sql("last_time");
            value_builder.push("?", last_time);
        }
        B::prepare(
            B::new_sql("insert into tb_config")
                .push_build(&mut field_builder)
                .push_sql("values")
                .push_build(&mut value_builder),
        )
    }

    pub fn update_prepare(
        &self,
        record: &ConfigDO,
        param: &ConfigParam,
    ) -> (String, Vec<serde_json::Value>) {
        let mut set_builder = B::new_comma();
        if let Some(id) = &record.id {
            set_builder.eq("id", id);
        }
        if let Some(data_id) = &record.data_id {
            set_builder.eq("data_id", data_id);
        }
        if let Some(group) = &record.group {
            set_builder.eq("`group`", group);
        }
        if let Some(tenant) = &record.tenant {
            set_builder.eq("tenant", tenant);
        }
        if let Some(content) = &record.content {
            set_builder.eq("content", content);
        }
        if let Some(content_md5) = &record.content_md5 {
            set_builder.eq("content_md5", content_md5);
        }
        if let Some(last_time) = &record.last_time {
            set_builder.eq("last_time", last_time);
        }
        let mut whr = self.conditions(param);
        if whr.is_empty() {
            panic!("update conditions is empty");
        }
        B::prepare(
            B::new_sql("update tb_config set ")
                .push_build(&mut set_builder)
                .push_build(&mut whr),
        )
    }

    pub fn delete_prepare(&self, param: &ConfigParam) -> (String, Vec<serde_json::Value>) {
        B::prepare(B::new_sql("delete from tb_config").push_build(&mut self.conditions(param)))
    }
}

pub struct ConfigDao {
    conn: Rc<Connection>,
    inner: ConfigSql,
}

impl ConfigDao {
    pub fn new(conn: Rc<Connection>) -> Self {
        Self {
            conn,
            inner: ConfigSql {},
        }
    }

    pub fn execute(&self, sql: &str, args: &[serde_json::Value]) -> anyhow::Result<usize> {
        sqlite_execute(&self.conn, sql, args)
    }

    pub fn fetch(&self, sql: &str, args: &[serde_json::Value]) -> anyhow::Result<Vec<ConfigDO>> {
        sqlite_fetch(&self.conn, sql, args, ConfigDO::from_row)
    }

    pub fn insert(&self, record: &ConfigDO) -> anyhow::Result<usize> {
        let (sql, args) = self.inner.insert_prepare(record);
        self.execute(&sql, &args)
    }

    pub fn update(&self, record: &ConfigDO, param: &ConfigParam) -> anyhow::Result<usize> {
        let (sql, args) = self.inner.update_prepare(record, param);
        self.execute(&sql, &args)
    }

    pub fn delete(&self, param: &ConfigParam) -> anyhow::Result<usize> {
        let (sql, args) = self.inner.delete_prepare(param);
        self.execute(&sql, &args)
    }

    pub fn query(&self, param: &ConfigParam) -> anyhow::Result<Vec<ConfigDO>> {
        let (sql, args) = self.inner.query_prepare(param);
        self.fetch(&sql, &args)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ConfigHistoryDO {
    pub id: Option<i64>,
    pub data_id: Option<String>,
    pub group: Option<String>,
    pub tenant: Option<String>,
    pub content: Option<String>,
    pub last_time: Option<i64>,
}

impl ConfigHistoryDO {
    fn from_row(r: &Row) -> Self {
        Self {
            id: get_row_value(r, "id"),
            data_id: get_row_value(r, "data_id"),
            group: get_row_value(r, "group"),
            tenant: get_row_value(r, "tenant"),
            content: get_row_value(r, "content"),
            last_time: get_row_value(r, "last_time"),
        }
    }
}

#[derive(Debug, Default)]
pub struct ConfigHistoryParam {
    pub id: Option<i64>,
    pub data_id: Option<String>,
    pub group: Option<String>,
    pub tenant: Option<String>,
    pub order_by: Option<String>,
    pub order_by_desc: Option<bool>,
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
        if let Some(group) = &param.group {
            whr.eq("`group`", group);
        }
        if let Some(tenant) = &param.tenant {
            whr.eq("tenant", tenant);
        }
        whr
    }

    fn offset_conditions(&self, param: &ConfigHistoryParam) -> B<'_> {
        let mut whr = B::new();
        if let Some(field) = &param.order_by {
            let desc = param.order_by_desc.to_owned().unwrap_or(false);
            if field.eq_ignore_ascii_case("last_time") {
                whr.order_by("last_time", desc);
            }
            if field.eq_ignore_ascii_case("id") {
                whr.order_by("id", desc);
            }
        }
        if let Some(limit) = &param.limit {
            whr.limit(limit);
        }
        if let Some(offset) = &param.offset {
            whr.offset(offset);
        }
        whr
    }

    pub fn query_prepare(&self, param: &ConfigHistoryParam) -> (String, Vec<serde_json::Value>) {
        B::new_sql("select id, data_id, `group`, tenant, content, last_time from tb_config_history")
            .push_build(&mut self.conditions(param))
            .push_build(&mut self.offset_conditions(param))
            .build()
    }

    pub fn query_count_prepare(
        &self,
        param: &ConfigHistoryParam,
    ) -> (String, Vec<serde_json::Value>) {
        B::prepare(
            B::new_sql("select count(1) from tb_config_history")
                .push_build(&mut self.conditions(param)),
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
        if let Some(group) = &record.group {
            field_builder.push_sql("`group`");
            value_builder.push("?", group);
        }
        if let Some(tenant) = &record.tenant {
            field_builder.push_sql("tenant");
            value_builder.push("?", tenant);
        }
        if let Some(content) = &record.content {
            field_builder.push_sql("content");
            value_builder.push("?", content);
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
        if let Some(group) = &record.group {
            set_builder.eq("`group`", group);
        }
        if let Some(tenant) = &record.tenant {
            set_builder.eq("tenant", tenant);
        }
        if let Some(content) = &record.content {
            set_builder.eq("content", content);
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

pub struct ConfigHistoryDao {
    conn: Rc<Connection>,
    inner: ConfigHistorySql,
}

impl ConfigHistoryDao {
    pub fn new(conn: Rc<Connection>) -> Self {
        Self {
            conn,
            inner: ConfigHistorySql {},
        }
    }

    pub fn execute(&self, sql: &str, args: &[serde_json::Value]) -> anyhow::Result<usize> {
        sqlite_execute(&self.conn, sql, args)
    }

    pub fn fetch(
        &self,
        sql: &str,
        args: &[serde_json::Value],
    ) -> anyhow::Result<Vec<ConfigHistoryDO>> {
        sqlite_fetch(&self.conn, sql, args, ConfigHistoryDO::from_row)
    }

    pub fn fetch_count(&self, sql: &str, args: &[serde_json::Value]) -> anyhow::Result<u64> {
        sqlite_fetch_count(&self.conn, sql, args)
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

    pub fn query_count(&self, param: &ConfigHistoryParam) -> anyhow::Result<u64> {
        let (sql, args) = self.inner.query_count_prepare(param);
        self.fetch_count(&sql, &args)
    }
}
