use serde_json::Value;
use sqlx::database::HasArguments;
use sqlx::mysql::MySqlRow;
use sqlx::query::Query;
use sqlx::{Executor, MySql, MySqlConnection, Transaction};

pub fn build_mysql_query<'a>(
    sql: &'a str,
    args: &'a [serde_json::Value],
) -> Query<'a, MySql, <MySql as HasArguments<'a>>::Arguments> {
    let mut q = sqlx::query(sql);
    for arg in args {
        match arg {
            Value::Number(v) => {
                if v.is_f64() {
                    q = q.bind(v.as_f64().unwrap())
                } else {
                    q = q.bind(v.as_i64().unwrap())
                }
            }
            Value::Bool(v) => q = q.bind(Some(v.to_owned())),
            Value::Null => q = q.bind(Option::<String>::None),
            Value::String(v) => {
                q = q.bind(Some(v.to_owned()));
            }
            _ => {
                q = q.bind(Some(arg.to_string()));
            }
        }
    }
    q
}

pub enum MySqlExecutor<'c> {
    Pool(&'c mut sqlx::Pool<MySql>),
    Conn(&'c mut MySqlConnection),
    Transaction(&'c mut Transaction<'c, MySql>),
}

#[derive(Debug, Clone)]
pub struct ExecuteResult {
    pub rows_affected: u64,
    pub last_insert_id: u64,
}

impl ExecuteResult {
    pub fn new(rows_affected: u64, last_insert_id: u64) -> ExecuteResult {
        Self {
            rows_affected,
            last_insert_id,
        }
    }
}

impl<'c> MySqlExecutor<'c> {
    pub fn new_by_pool(pool: &'c mut sqlx::Pool<MySql>) -> Self {
        Self::Pool(pool)
    }
    pub fn new_by_conn(conn: &'c mut MySqlConnection) -> Self {
        Self::Conn(conn)
    }
    pub fn new_by_transaction(tx: &'c mut Transaction<'c, MySql>) -> Self {
        Self::Transaction(tx)
    }

    pub async fn fetch<T>(
        &mut self,
        sql: &str,
        args: &[serde_json::Value],
    ) -> anyhow::Result<Vec<T>>
    where
        T: for<'r> sqlx::FromRow<'r, MySqlRow> + Send + Unpin,
    {
        let q = build_mysql_query(sql, args);
        match self {
            MySqlExecutor::Pool(executor) => {
                let res = executor.fetch_all(q).await?;
                let rlist: Vec<T> = res.into_iter().map(|e| T::from_row(&e).unwrap()).collect();
                Ok(rlist)
            }
            MySqlExecutor::Conn(executor) => {
                let res = executor.fetch_all(q).await?;
                let rlist: Vec<T> = res.into_iter().map(|e| T::from_row(&e).unwrap()).collect();
                Ok(rlist)
            }
            MySqlExecutor::Transaction(executor) => {
                let res = executor.fetch_all(q).await?;
                let rlist: Vec<T> = res.into_iter().map(|e| T::from_row(&e).unwrap()).collect();
                Ok(rlist)
            }
        }
    }

    pub async fn fetch_one<T>(&mut self, sql: &str, args: &[serde_json::Value]) -> anyhow::Result<T>
    where
        T: for<'r> sqlx::FromRow<'r, MySqlRow> + Send + Unpin,
    {
        let q = build_mysql_query(sql, args);
        match self {
            MySqlExecutor::Pool(executor) => {
                let res = executor.fetch_one(q).await?;
                Ok(T::from_row(&res)?)
            }
            MySqlExecutor::Conn(executor) => {
                let res = executor.fetch_one(q).await?;
                Ok(T::from_row(&res)?)
            }
            MySqlExecutor::Transaction(executor) => {
                let res = executor.fetch_one(q).await?;
                Ok(T::from_row(&res)?)
            }
        }
    }

    pub async fn fetch_row(
        &mut self,
        sql: &str,
        args: &[serde_json::Value],
    ) -> anyhow::Result<MySqlRow> {
        let q = build_mysql_query(sql, args);
        match self {
            MySqlExecutor::Pool(executor) => {
                let res = executor.fetch_one(q).await?;
                Ok(res)
            }
            MySqlExecutor::Conn(executor) => {
                let res = executor.fetch_one(q).await?;
                Ok(res)
            }
            MySqlExecutor::Transaction(executor) => {
                let res = executor.fetch_one(q).await?;
                Ok(res)
            }
        }
    }

    pub async fn execute(
        &mut self,
        sql: &str,
        args: &[serde_json::Value],
    ) -> anyhow::Result<ExecuteResult> {
        let q = build_mysql_query(sql, args);
        match self {
            MySqlExecutor::Pool(executor) => {
                let res = executor.execute(q).await?;
                Ok(ExecuteResult::new(
                    res.rows_affected(),
                    res.last_insert_id(),
                ))
            }
            MySqlExecutor::Conn(executor) => {
                let res = executor.execute(q).await?;
                Ok(ExecuteResult::new(
                    res.rows_affected(),
                    res.last_insert_id(),
                ))
            }
            MySqlExecutor::Transaction(executor) => {
                let res = executor.execute(q).await?;
                Ok(ExecuteResult::new(
                    res.rows_affected(),
                    res.last_insert_id(),
                ))
            }
        }
    }
}
