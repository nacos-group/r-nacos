#[allow(dead_code, unused_imports)]
use rsql_builder::B;
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};

use crate::common::rusqlite_utils::{
    get_row_value, sqlite_execute, sqlite_fetch, sqlite_fetch_count,
};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TenantDO {
    pub id: Option<i64>,
    pub tenant_id: Option<String>,
    pub tenant_name: Option<String>,
    pub tenant_desc: Option<String>,
    pub create_flag: Option<i64>,
}

impl TenantDO {
    fn from_row(r: &Row) -> Self {
        let mut s = Self::default();
        s.id = get_row_value(r, "id");
        s.tenant_id = get_row_value(r, "tenant_id");
        s.tenant_name = get_row_value(r, "tenant_name");
        s.tenant_desc = get_row_value(r, "tenant_desc");
        s.create_flag = get_row_value(r, "create_flag");
        s
    }
}

#[derive(Debug, Default)]
pub struct TenantParam {
    pub id: Option<i64>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
pub struct TenantSql {}

impl TenantSql {
    fn conditions(&self, param: &TenantParam) -> B<'_> {
        let mut whr = B::new_where();
        if let Some(id) = &param.id {
            whr.eq("id", id);
        }
        whr
    }

    pub fn query_prepare(&self, param: &TenantParam) -> (String, Vec<serde_json::Value>) {
        B::prepare(
            B::new_sql(
                "select id, tenant_id, tenant_name, tenant_desc, create_flag from tb_tenant",
            )
            .push_build(&mut self.conditions(param)),
        )
    }

    pub fn insert_prepare(&self, record: &TenantDO) -> (String, Vec<serde_json::Value>) {
        let mut field_builder = B::new_comma_paren();
        let mut value_builder = B::new_comma_paren();
        if let Some(id) = &record.id {
            field_builder.push_sql("id");
            value_builder.push("?", id);
        }
        if let Some(tenant_id) = &record.tenant_id {
            field_builder.push_sql("tenant_id");
            value_builder.push("?", tenant_id);
        }
        if let Some(tenant_name) = &record.tenant_name {
            field_builder.push_sql("tenant_name");
            value_builder.push("?", tenant_name);
        }
        if let Some(tenant_desc) = &record.tenant_desc {
            field_builder.push_sql("tenant_desc");
            value_builder.push("?", tenant_desc);
        }
        if let Some(create_flag) = &record.create_flag {
            field_builder.push_sql("create_flag");
            value_builder.push("?", create_flag);
        }
        B::prepare(
            B::new_sql("insert into tb_tenant")
                .push_build(&mut field_builder)
                .push_sql("values")
                .push_build(&mut value_builder),
        )
    }

    pub fn update_prepare(
        &self,
        record: &TenantDO,
        param: &TenantParam,
    ) -> (String, Vec<serde_json::Value>) {
        let mut set_builder = B::new_comma();
        if let Some(id) = &record.id {
            set_builder.eq("id", id);
        }
        if let Some(tenant_id) = &record.tenant_id {
            set_builder.eq("tenant_id", tenant_id);
        }
        if let Some(tenant_name) = &record.tenant_name {
            set_builder.eq("tenant_name", tenant_name);
        }
        if let Some(tenant_desc) = &record.tenant_desc {
            set_builder.eq("tenant_desc", tenant_desc);
        }
        if let Some(create_flag) = &record.create_flag {
            set_builder.eq("create_flag", create_flag);
        }
        let mut whr = self.conditions(param);
        if whr.is_empty() {
            panic!("update conditions is empty");
        }
        B::prepare(
            B::new_sql("update tb_tenant set ")
                .push_build(&mut set_builder)
                .push_build(&mut whr),
        )
    }

    pub fn delete_prepare(&self, param: &TenantParam) -> (String, Vec<serde_json::Value>) {
        B::prepare(B::new_sql("delete from tb_tenant").push_build(&mut self.conditions(param)))
    }
}

pub struct TenantDao<'a> {
    conn: &'a Connection,
    inner: TenantSql,
}

impl<'a> TenantDao<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self {
            conn,
            inner: TenantSql {},
        }
    }

    pub fn execute(&self, sql: &str, args: &[serde_json::Value]) -> anyhow::Result<usize> {
        sqlite_execute(self.conn, sql, args)
    }

    pub fn fetch(&self, sql: &str, args: &[serde_json::Value]) -> anyhow::Result<Vec<TenantDO>> {
        sqlite_fetch(self.conn, sql, args, TenantDO::from_row)
    }

    pub fn fetch_count(&self, sql: &str, args: &[serde_json::Value]) -> anyhow::Result<u64> {
        sqlite_fetch_count(self.conn, sql, args)
    }

    pub fn insert(&self, record: &TenantDO) -> anyhow::Result<usize> {
        let (sql, args) = self.inner.insert_prepare(record);
        self.execute(&sql, &args)
    }

    pub fn update(&self, record: &TenantDO, param: &TenantParam) -> anyhow::Result<usize> {
        let (sql, args) = self.inner.update_prepare(record, param);
        self.execute(&sql, &args)
    }

    pub fn delete(&self, param: &TenantParam) -> anyhow::Result<usize> {
        let (sql, args) = self.inner.delete_prepare(param);
        self.execute(&sql, &args)
    }

    pub fn query(&self, param: &TenantParam) -> anyhow::Result<Vec<TenantDO>> {
        let (sql, args) = self.inner.query_prepare(param);
        self.fetch(&sql, &args)
    }
}
