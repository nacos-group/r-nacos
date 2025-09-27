use crate::common::sqlx_utils::MySqlExecutor;
use rsql_builder::B;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;

#[derive(Debug, Default, Serialize, Deserialize, sqlx::FromRow)]
pub struct ConfigHistoryDO {
    pub id: Option<u64>,
    pub nid: Option<u64>,
    pub data_id: Option<String>,
    pub group_id: Option<String>,
    pub app_name: Option<String>,
    pub content: Option<String>,
    pub md5: Option<String>,
    pub gmt_create_timestamp: Option<i64>,
    pub src_user: Option<String>,
    pub src_ip: Option<String>,
    pub op_type: Option<String>,
    pub tenant_id: Option<String>,
    pub encrypted_data_key: Option<String>,
}

#[derive(Debug, Default)]
pub struct ConfigHistoryParam {
    pub id: Option<i64>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub data_id: Option<Arc<String>>,
    pub group_id: Option<Arc<String>>,
    pub tenant_id: Option<Arc<String>>,
    pub order_by_gmt_create_desc: bool,
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
            B::new_sql(
                "select id, nid, data_id, group_id, app_name, content, md5, \
            UNIX_TIMESTAMP(gmt_create) as gmt_create_timestamp, src_user, src_ip, op_type, \
            tenant_id, encrypted_data_key from his_config_info",
            )
            .push_build(&mut self.conditions(param))
            .push_fn(|| {
                let mut b = B::new();
                if param.order_by_gmt_create_desc {
                    b.order_by("gmt_create", true);
                }
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

    pub fn query_count_prepare(
        &self,
        param: &ConfigHistoryParam,
    ) -> (String, Vec<serde_json::Value>) {
        B::prepare(
            B::new_sql("select count(1) from his_config_info")
                .push_build(&mut self.conditions(param)),
        )
    }
}

pub struct ConfigHistoryDao<'a> {
    executor: MySqlExecutor<'a>,
    inner: ConfigHistorySql,
}

impl<'a> ConfigHistoryDao<'a> {
    pub fn new(executor: MySqlExecutor<'a>) -> Self {
        Self {
            executor,
            inner: ConfigHistorySql {},
        }
    }

    pub async fn fetch_count(
        &mut self,
        sql: &str,
        args: &[serde_json::Value],
    ) -> anyhow::Result<i64> {
        let v = self.executor.fetch_row(sql, args).await?;
        v.try_get(0).map_err(anyhow::Error::msg)
    }

    pub async fn fetch(
        &mut self,
        sql: &str,
        args: &[serde_json::Value],
    ) -> anyhow::Result<Vec<ConfigHistoryDO>> {
        self.executor.fetch(sql, args).await
    }

    pub async fn query(
        &mut self,
        param: &ConfigHistoryParam,
    ) -> anyhow::Result<Vec<ConfigHistoryDO>> {
        let (sql, args) = self.inner.query_prepare(param);
        self.fetch(&sql, &args).await
    }

    pub async fn query_count(&mut self, param: &ConfigHistoryParam) -> anyhow::Result<i64> {
        let (sql, args) = self.inner.query_count_prepare(param);
        self.fetch_count(&sql, &args).await
    }
}
