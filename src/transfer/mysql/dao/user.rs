use crate::common::sqlx_utils::MySqlExecutor;
use rsql_builder::B;
use serde::{Deserialize, Serialize};
use sqlx::Row;

#[derive(Debug, Default, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserDO {
    pub username: Option<String>,
    pub password: Option<String>,
    pub enabled: Option<i64>,
}

#[derive(Debug, Default)]
pub struct UserParam {
    pub id: Option<i64>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
pub struct UserSql {}

impl UserSql {
    fn conditions(&self, param: &UserParam) -> B<'_> {
        let mut whr = B::new_where();
        if let Some(id) = &param.id {
            whr.eq("id", id);
        }
        whr
    }

    pub fn query_prepare(&self, param: &UserParam) -> (String, Vec<serde_json::Value>) {
        B::prepare(
            B::new_sql("select username, password, enabled from users")
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

    pub fn query_count_prepare(&self, param: &UserParam) -> (String, Vec<serde_json::Value>) {
        B::prepare(B::new_sql("select count(1) from users").push_build(&mut self.conditions(param)))
    }
}

pub struct UserDao<'a> {
    executor: MySqlExecutor<'a>,
    inner: UserSql,
}

impl<'a> UserDao<'a> {
    pub fn new(executor: MySqlExecutor<'a>) -> Self {
        Self {
            executor,
            inner: UserSql {},
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
    ) -> anyhow::Result<Vec<UserDO>> {
        self.executor.fetch(sql, args).await
    }

    pub async fn query(&mut self, param: &UserParam) -> anyhow::Result<Vec<UserDO>> {
        let (sql, args) = self.inner.query_prepare(param);
        self.fetch(&sql, &args).await
    }

    pub async fn query_count(&mut self, param: &UserParam) -> anyhow::Result<i64> {
        let (sql, args) = self.inner.query_count_prepare(param);
        self.fetch_count(&sql, &args).await
    }
}
