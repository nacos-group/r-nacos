use rusqlite::{Connection,Row,params_from_iter};

fn result2option<T>(r:rusqlite::Result<T>) -> Option<T> {
    match r {
        Ok(v) => Some(v),
        _ => None,
    }
}

pub fn get_row_value<T>(r:&Row,name:&str) -> Option<T> 
where T: rusqlite::types::FromSql 
{
    match r.column_index(name) {
        Ok(i) => result2option(r.get(i)),
        _ => None,
    }
}

pub fn convert_json_param(val:&serde_json::Value) -> rusqlite::types::ToSqlOutput<'_> {
    match val {
        serde_json::Value::Null => {
            rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Null)
        },
        serde_json::Value::Number(n) => {
            if n.is_f64(){
                return rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Real(n.as_f64().unwrap()));
            }
            rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Integer(n.as_i64().unwrap()))
        },
        serde_json::Value::String(s)=> {
            rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Text(s.to_owned()))
        },
        _ => rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Text(val.to_string()))
    }
}

pub fn convert_json_params(inputs:&Vec<serde_json::Value>) -> Vec<rusqlite::types::ToSqlOutput<'_>> {
    inputs.iter().map(|e|convert_json_param(e)).collect()
}


pub fn sqlite_execute(conn:&Connection,sql:&str,args:&Vec<serde_json::Value>) -> Result<usize,String> {
    //println!("sqlite_execute, {} | {:?}",&sql,&args);
    let result = conn.execute(&sql,params_from_iter(
        convert_json_params(args).iter()) );
    match result {
        Ok(v) => Ok(v),
        Err(e) => Err(e.to_string())
    }
}

pub fn sqlite_fetch<T,F>(conn:&Connection,sql:&str,args:&Vec<serde_json::Value>,convert:F) -> Result<Vec<T>,String> 
where F: Fn(&Row) -> T + Send
{
    //println!("sqlite_fetch, {} | {:?}",&sql,&args);
    let mut stmt = conn.prepare(&sql).unwrap();
        let r = stmt.query_map(params_from_iter(
            convert_json_params(&args).iter()), |r|{Ok(convert(r))});
        match r {
            Ok(res) => {
                let list:Vec<T>=res.into_iter().map(|e|e.unwrap()).collect();
                Ok(list)
            },
            Err(e) => Err(e.to_string())
        }
}

pub fn sqlite_fetch_count(conn:&Connection,sql:&str,args:&Vec<serde_json::Value>) -> Result<u64,String> 
{
    let mut stmt = conn.prepare(&sql).unwrap();
        let r = stmt.query_map(params_from_iter(
            convert_json_params(&args).iter()), |r|{r.get(0)});
        match r {
            Ok(res) => {
                let list:Vec<u64>=res.into_iter().map(|e|e.unwrap()).collect();
                let r:u64=list.get(0).unwrap().to_owned();
                Ok(r)
            },
            Err(e) => Err(e.to_string())
        }
}