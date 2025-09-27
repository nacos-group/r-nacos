use crate::common::sqlx_utils::MySqlExecutor;
use rsql_builder::B;
use serde::{Deserialize, Serialize};
use sqlx::Row;

#[derive(Debug, Default, Serialize, Deserialize, sqlx::FromRow)]
pub struct TenantDO {
    pub id: Option<i64>,
    pub tenant_id: Option<String>,
    pub tenant_name: Option<String>,
    pub tenant_desc: Option<String>,
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
            B::new_sql("select id, tenant_id, tenant_name, tenant_desc from tenant_info")
                .push_build(&mut self.conditions(param))
                .push_fn(|| {
                    let mut b = B::new();
                    if let Some(limit) = &param.limit {
                        b.limit(limit);
                    }
                    if let Some(offset) = &param.offset {
                        b.offset(offset);
                    }
                    b
                }),
        )
    }

    pub fn query_count_prepare(&self, param: &TenantParam) -> (String, Vec<serde_json::Value>) {
        B::prepare(
            B::new_sql("select count(1) from tenant_info").push_build(&mut self.conditions(param)),
        )
    }
}

pub struct TenantDao<'a> {
    executor: MySqlExecutor<'a>,
    inner: TenantSql,
}

impl<'a> TenantDao<'a> {
    pub fn new(executor: MySqlExecutor<'a>) -> Self {
        Self {
            executor,
            inner: TenantSql {},
        }
    }
    pub async fn fetch(
        &mut self,
        sql: &str,
        args: &[serde_json::Value],
    ) -> anyhow::Result<Vec<TenantDO>> {
        self.executor.fetch(sql, args).await
    }

    pub async fn fetch_count(
        &mut self,
        sql: &str,
        args: &[serde_json::Value],
    ) -> anyhow::Result<i64> {
        let v = self.executor.fetch_row(sql, args).await?;
        v.try_get(0).map_err(anyhow::Error::msg)
    }

    pub async fn query(&mut self, param: &TenantParam) -> anyhow::Result<Vec<TenantDO>> {
        let (sql, args) = self.inner.query_prepare(param);
        self.fetch(&sql, &args).await
    }

    pub async fn query_count(&mut self, param: &TenantParam) -> anyhow::Result<i64> {
        let (sql, args) = self.inner.query_count_prepare(param);
        self.fetch_count(&sql, &args).await
    }
}
