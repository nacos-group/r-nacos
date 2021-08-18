use rsql_builder::B;
use rusqlite::{Connection,Row};
use serde::{Serialize,Deserialize};
use std::rc::Rc;

use crate::rusqlite_utils::{get_row_value,sqlite_execute,sqlite_fetch};

#[derive(Debug,Default,Serialize,Deserialize)]
pub struct ConfigDO {
    pub id:Option<i64>,
    pub data_id:Option<String>,
    pub group:Option<String>,
    pub tenant:Option<String>,
    pub content:Option<String>,
    pub content_md5:Option<String>,
    pub last_time:Option<i64>,
}

impl ConfigDO {
    fn from_row(r:&Row) -> Self {
        let mut s = Self::default();
        s.id = get_row_value(r,"id");
        s.data_id = get_row_value(r,"data_id");
        s.group = get_row_value(r,"group");
        s.tenant = get_row_value(r,"tenant");
        s.content = get_row_value(r,"content");
        s.content_md5 = get_row_value(r,"content_md5");
        s.last_time = get_row_value(r,"last_time");
        s
    }
}

#[derive(Debug,Default)]
pub struct ConfigParam{
    pub id:Option<i64>,
    pub data_id:Option<String>,
    pub group:Option<String>,
    pub tenant:Option<String>,
    pub limit:Option<i64>,
    pub offset:Option<i64>,
}
pub struct ConfigSql{}

impl ConfigSql{
    fn conditions(&self,param:&ConfigParam) -> B {
        let mut whr = B::new_where();
        if let Some(id)=&param.id {
            whr.eq("id",id);
        }
        if let Some(data_id)=&param.data_id {
            whr.eq("data_id",data_id);
        }
        if let Some(group)=&param.group {
            whr.eq("`group`",group);
        }
        if let Some(tenant)=&param.tenant {
            whr.eq("tenant",tenant);
        }
        whr
    }

    pub fn query_prepare(&self,param:&ConfigParam) -> (String,Vec<serde_json::Value>) {
        B::prepare(
            B::new_sql("select id, data_id, `group`, tenant, content, content_md5, last_time tb_config")
            .push_build(&mut self.conditions(param))
        )
    }

    pub fn insert_prepare(&self,record:&ConfigDO) -> (String,Vec<serde_json::Value>) {
        let mut field_builder=B::new_comma_paren();
        let mut value_builder=B::new_comma_paren();
        if let Some(id) = &record.id {
            field_builder.push_sql("id");
            value_builder.push("?",id);
        }
        if let Some(data_id) = &record.data_id {
            field_builder.push_sql("data_id");
            value_builder.push("?",data_id);
        }
        if let Some(group) = &record.group {
            field_builder.push_sql("`group`");
            value_builder.push("?",group);
        }
        if let Some(tenant) = &record.tenant {
            field_builder.push_sql("tenant");
            value_builder.push("?",tenant);
        }
        if let Some(content) = &record.content {
            field_builder.push_sql("content");
            value_builder.push("?",content);
        }
        if let Some(content_md5) = &record.content_md5 {
            field_builder.push_sql("content_md5");
            value_builder.push("?",content_md5);
        }
        if let Some(last_time) = &record.last_time {
            field_builder.push_sql("last_time");
            value_builder.push("?",last_time);
        }
        B::prepare(
            B::new_sql("insert into tb_config")
            .push_build(&mut field_builder)
            .push_sql("values")
            .push_build(&mut value_builder)
        )
    }

    pub fn update_prepare(&self,record:&ConfigDO) -> (String,Vec<serde_json::Value>) {
        let mut set_builder=B::new_comma();
        if let Some(id) = &record.id {
            set_builder.eq("id",id);
        }
        if let Some(data_id) = &record.data_id {
            set_builder.eq("data_id",data_id);
        }
        if let Some(group) = &record.group {
            set_builder.eq("`group`",group);
        }
        if let Some(tenant) = &record.tenant {
            set_builder.eq("tenant",tenant);
        }
        if let Some(content) = &record.content {
            set_builder.eq("content",content);
        }
        if let Some(content_md5) = &record.content_md5 {
            set_builder.eq("content_md5",content_md5);
        }
        if let Some(last_time) = &record.last_time {
            set_builder.eq("last_time",last_time);
        }
        let mut whr = B::new_where();
        if let Some(id)=&record.id {
            whr.eq("id",id);
        }
        if whr.is_empty() {
            panic!("update conditions is empty");
        }
        B::prepare(
             B::new_sql("update tb_config set ")
            .push_build(&mut set_builder)
            .push_build(&mut whr)
        )
    }

    pub fn delete_prepare(&self,param:&ConfigParam) -> (String,Vec<serde_json::Value>) {
        B::prepare(
            B::new_sql("delete from tb_config")
            .push_build(&mut self.conditions(param))
        )
    }
}

pub struct ConfigDao{
    conn: Rc<Connection>,
    inner: ConfigSql,
}

impl ConfigDao {
    pub fn new(conn: Rc<Connection>) -> Self{
        Self{ 
            conn:conn,
            inner:ConfigSql{},
        }
    }

    pub fn execute(&self,sql:&str,args:&Vec<serde_json::Value>) -> Result<usize,String>{
        sqlite_execute(&self.conn,sql,args) 
    }

    pub fn fetch(&self,sql:&str,args:&Vec<serde_json::Value>) -> Vec<ConfigDO> {
        sqlite_fetch(&self.conn,sql,args,ConfigDO::from_row).unwrap()
    }

    pub fn insert(&self,record:&ConfigDO) -> Result<usize,String> {
        let (sql,args) = self.inner.insert_prepare(record);
        self.execute(&sql, &args)
    }

    pub fn update(&self,record:&ConfigDO) -> Result<usize,String> {
        let (sql,args) = self.inner.update_prepare(record);
        self.execute(&sql, &args)
    }

    pub fn delete(&self,param:&ConfigParam) -> Result<usize,String> {
        let (sql,args) = self.inner.delete_prepare(param);
        self.execute(&sql, &args)
    }

    pub fn query(&self,param:&ConfigParam) -> Vec<ConfigDO> {
        let (sql,args) = self.inner.query_prepare(param);
        self.fetch(&sql, &args)
    }
}

#[derive(Debug,Default,Serialize,Deserialize)]
pub struct ConfigHistoryDO {
    pub id:Option<i64>,
    pub data_id:Option<String>,
    pub group:Option<String>,
    pub tenant:Option<String>,
    pub content:Option<String>,
    pub last_time:Option<i64>,
}

impl ConfigHistoryDO {
    fn from_row(r:&Row) -> Self {
        let mut s = Self::default();
        s.id = get_row_value(r,"id");
        s.data_id = get_row_value(r,"data_id");
        s.group = get_row_value(r,"group");
        s.tenant = get_row_value(r,"tenant");
        s.content = get_row_value(r,"content");
        s.last_time = get_row_value(r,"last_time");
        s
    }
}

#[derive(Debug,Default)]
pub struct ConfigHistoryParam{
    pub id:Option<i64>,
    pub data_id:Option<String>,
    pub group:Option<String>,
    pub tenant:Option<String>,
    pub limit:Option<i64>,
    pub offset:Option<i64>,
}
pub struct ConfigHistorySql{}

impl ConfigHistorySql{
    fn conditions(&self,param:&ConfigHistoryParam) -> B {
        let mut whr = B::new_where();
        if let Some(id)=&param.id {
            whr.eq("id",id);
        }
        if let Some(data_id)=&param.data_id {
            whr.eq("data_id",data_id);
        }
        if let Some(group)=&param.group {
            whr.eq("`group`",group);
        }
        if let Some(tenant)=&param.tenant {
            whr.eq("tenant",tenant);
        }
        whr
    }

    pub fn query_prepare(&self,param:&ConfigHistoryParam) -> (String,Vec<serde_json::Value>) {
        B::prepare(
            B::new_sql("select id, data_id, group, tenant, content, last_time tb_config_history")
            .push_build(&mut self.conditions(param))
        )
    }

    pub fn insert_prepare(&self,record:&ConfigHistoryDO) -> (String,Vec<serde_json::Value>) {
        let mut field_builder=B::new_comma_paren();
        let mut value_builder=B::new_comma_paren();
        if let Some(id) = &record.id {
            field_builder.push_sql("id");
            value_builder.push("?",id);
        }
        if let Some(data_id) = &record.data_id {
            field_builder.push_sql("data_id");
            value_builder.push("?",data_id);
        }
        if let Some(group) = &record.group {
            field_builder.push_sql("`group`");
            value_builder.push("?",group);
        }
        if let Some(tenant) = &record.tenant {
            field_builder.push_sql("tenant");
            value_builder.push("?",tenant);
        }
        if let Some(content) = &record.content {
            field_builder.push_sql("content");
            value_builder.push("?",content);
        }
        if let Some(last_time) = &record.last_time {
            field_builder.push_sql("last_time");
            value_builder.push("?",last_time);
        }
        B::prepare(
            B::new_sql("insert into tb_config_history")
            .push_build(&mut field_builder)
            .push_sql("values")
            .push_build(&mut value_builder)
        )
    }

    pub fn update_prepare(&self,record:&ConfigHistoryDO) -> (String,Vec<serde_json::Value>) {
        let mut set_builder=B::new_comma();
        if let Some(id) = &record.id {
            set_builder.eq("id",id);
        }
        if let Some(data_id) = &record.data_id {
            set_builder.eq("data_id",data_id);
        }
        if let Some(group) = &record.group {
            set_builder.eq("`group`",group);
        }
        if let Some(tenant) = &record.tenant {
            set_builder.eq("tenant",tenant);
        }
        if let Some(content) = &record.content {
            set_builder.eq("content",content);
        }
        if let Some(last_time) = &record.last_time {
            set_builder.eq("last_time",last_time);
        }
        let mut whr = B::new_where();
        if let Some(id)=&record.id {
            whr.eq("id",id);
        }
        if whr.is_empty() {
            panic!("update conditions is empty");
        }
        B::prepare(
             B::new_sql("update tb_config_history set ")
            .push_build(&mut set_builder)
            .push_build(&mut whr)
        )
    }

    pub fn delete_prepare(&self,param:&ConfigHistoryParam) -> (String,Vec<serde_json::Value>) {
        B::prepare(
            B::new_sql("delete from tb_config_history")
            .push_build(&mut self.conditions(param))
        )
    }
}

pub struct ConfigHistoryDao{
    conn: Rc<Connection>,
    inner: ConfigHistorySql,
}

impl ConfigHistoryDao {
    pub fn new(conn: Rc<Connection>) -> Self{
        Self{ 
            conn:conn,
            inner:ConfigHistorySql{},
        }
    }

    pub fn execute(&self,sql:&str,args:&Vec<serde_json::Value>) -> Result<usize,String>{
        sqlite_execute(&self.conn,sql,args) 
    }

    pub fn fetch(&self,sql:&str,args:&Vec<serde_json::Value>) -> Vec<ConfigHistoryDO> {
        sqlite_fetch(&self.conn,sql,args,ConfigHistoryDO::from_row).unwrap()
    }

    pub fn insert(&self,record:&ConfigHistoryDO) -> Result<usize,String> {
        let (sql,args) = self.inner.insert_prepare(record);
        self.execute(&sql, &args)
    }

    pub fn update(&self,record:&ConfigHistoryDO) -> Result<usize,String> {
        let (sql,args) = self.inner.update_prepare(record);
        self.execute(&sql, &args)
    }

    pub fn delete(&self,param:&ConfigHistoryParam) -> Result<usize,String> {
        let (sql,args) = self.inner.delete_prepare(param);
        self.execute(&sql, &args)
    }

    pub fn query(&self,param:&ConfigHistoryParam) -> Vec<ConfigHistoryDO> {
        let (sql,args) = self.inner.query_prepare(param);
        self.fetch(&sql, &args)
    }
}

