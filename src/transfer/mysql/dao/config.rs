use crate::common::sqlx_utils::MySqlExecutor;
use rsql_builder::B;
use serde::{Deserialize, Serialize};
use sqlx::Row;

#[derive(Debug, Default, Serialize, Deserialize, sqlx::FromRow)]
pub struct ConfigInfoDO {
    pub id: Option<i64>,
    pub data_id: Option<String>,
    pub group_id: Option<String>,
    pub content: Option<String>,
    pub md5: Option<String>,
    pub gmt_modified_timestamp: Option<i64>,
    pub src_user: Option<String>,
    pub src_ip: Option<String>,
    pub app_name: Option<String>,
    pub tenant_id: Option<String>,
    pub c_desc: Option<String>,
    pub c_use: Option<String>,
    pub effect: Option<String>,
    pub r#type: Option<String>,
    pub c_schema: Option<String>,
    pub encrypted_data_key: Option<String>,
}

#[derive(Debug, Default)]
pub struct ConfigInfoParam {
    pub id: Option<i64>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
pub struct ConfigInfoSql {}

impl ConfigInfoSql {
    fn conditions(&self, param: &ConfigInfoParam) -> B<'_> {
        let mut whr = B::new_where();
        if let Some(id) = &param.id {
            whr.eq("id", id);
        }
        whr
    }

    pub fn query_prepare(&self, param: &ConfigInfoParam) -> (String, Vec<serde_json::Value>) {
        B::prepare(
            B::new_sql("select id, data_id, group_id, content, md5, UNIX_TIMESTAMP(gmt_modified) \
            as gmt_modified_timestamp, src_user, src_ip, app_name, tenant_id, c_desc, c_use, effect, \
            type, c_schema, encrypted_data_key from config_info")
                .push_build(&mut self.conditions(param))
                .push_fn(||{
                    let mut b= B::new();
                    if let Some(limit) = &param.limit{
                        b.limit(limit);
                    }
                    if let Some(offset ) = &param.offset{
                        b.offset(offset);
                    }
                    b
                })
        )
    }

    pub fn query_count_prepare(&self, param: &ConfigInfoParam) -> (String, Vec<serde_json::Value>) {
        B::prepare(
            B::new_sql("select count(1) from config_info").push_build(&mut self.conditions(param)),
        )
    }
}

pub struct ConfigInfoDao<'a> {
    executor: MySqlExecutor<'a>,
    inner: ConfigInfoSql,
}

impl<'a> ConfigInfoDao<'a> {
    pub fn new(executor: MySqlExecutor<'a>) -> Self {
        Self {
            executor,
            inner: ConfigInfoSql {},
        }
    }

    pub async fn fetch(
        &mut self,
        sql: &str,
        args: &[serde_json::Value],
    ) -> anyhow::Result<Vec<ConfigInfoDO>> {
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

    pub async fn query(&mut self, param: &ConfigInfoParam) -> anyhow::Result<Vec<ConfigInfoDO>> {
        let (sql, args) = self.inner.query_prepare(param);
        self.fetch(&sql, &args).await
    }

    pub async fn query_count(&mut self, param: &ConfigInfoParam) -> anyhow::Result<i64> {
        let (sql, args) = self.inner.query_count_prepare(param);
        self.fetch_count(&sql, &args).await
    }
}
