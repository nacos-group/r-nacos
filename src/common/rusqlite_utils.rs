#![allow(dead_code, unused_imports)]

use std::sync::Arc;

use rusqlite::{params_from_iter, Connection, Row};

fn result2option<T>(r: rusqlite::Result<T>) -> Option<T> {
    r.ok()
}

fn result_to_arc_option<T>(r: rusqlite::Result<T>) -> Option<Arc<T>> {
    match r {
        Ok(v) => Some(Arc::new(v)),
        _ => None,
    }
}

pub fn get_row_value<T>(r: &Row, name: &str) -> Option<T>
where
    T: rusqlite::types::FromSql,
{
    match r.column_index(name) {
        Ok(i) => result2option(r.get(i)),
        _ => None,
    }
}

pub fn get_row_arc_value<T>(r: &Row, name: &str) -> Option<Arc<T>>
where
    T: rusqlite::types::FromSql,
{
    match r.column_index(name) {
        Ok(i) => result_to_arc_option(r.get(i)),
        _ => None,
    }
}

pub fn convert_json_param(val: &serde_json::Value) -> rusqlite::types::ToSqlOutput<'_> {
    match val {
        serde_json::Value::Null => {
            rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Null)
        }
        serde_json::Value::Number(n) => {
            if n.is_f64() {
                return rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Real(
                    n.as_f64().unwrap(),
                ));
            }
            rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Integer(
                n.as_i64().unwrap(),
            ))
        }
        serde_json::Value::String(s) => {
            rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Text(s.to_owned()))
        }
        _ => rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Text(val.to_string())),
    }
}

pub fn convert_json_params(inputs: &[serde_json::Value]) -> Vec<rusqlite::types::ToSqlOutput<'_>> {
    inputs.iter().map(convert_json_param).collect()
}

pub fn sqlite_execute(
    conn: &Connection,
    sql: &str,
    args: &[serde_json::Value],
) -> anyhow::Result<usize> {
    //println!("sqlite_execute, {} | {:?}",&sql,&args);
    let result = conn.execute(sql, params_from_iter(convert_json_params(args).iter()));
    Ok(result?)
}

pub fn sqlite_fetch<T, F>(
    conn: &Connection,
    sql: &str,
    args: &[serde_json::Value],
    convert: F,
) -> anyhow::Result<Vec<T>>
where
    F: Fn(&Row) -> T + Send,
{
    //println!("sqlite_fetch, {} | {:?}",&sql,&args);
    let mut stmt = conn.prepare(sql).unwrap();
    let r = stmt.query_map(params_from_iter(convert_json_params(args).iter()), |r| {
        Ok(convert(r))
    });
    let res = r?;
    let list: Vec<T> = res.into_iter().map(|e| e.unwrap()).collect();
    Ok(list)
}

pub fn sqlite_fetch_count(
    conn: &Connection,
    sql: &str,
    args: &[serde_json::Value],
) -> anyhow::Result<u64> {
    //println!("sqlite_fetch_count, {} | {:?}",&sql,&args);
    let mut stmt = conn.prepare(sql).unwrap();
    let r = stmt.query_map(params_from_iter(convert_json_params(args).iter()), |r| {
        r.get(0)
    });
    let res = r?;
    let list: Vec<u64> = res.into_iter().map(|e| e.unwrap()).collect();
    let v = list.first().unwrap().to_owned();
    Ok(v)
}
