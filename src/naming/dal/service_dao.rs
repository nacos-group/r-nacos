use std::sync::Arc;

use rsql_builder::B;
use rusqlite::Connection;
use crate::common::rusqlite_utils::{sqlite_execute,sqlite_fetch, sqlite_fetch_count};
use super::service_do::{ServiceParam, ServiceDO};

pub struct ServiceSql;

impl ServiceSql{
    fn conditions(&self,param:&ServiceParam) -> B {
        let mut whr = B::new_where();
        if let Some(id)=&param.id {
            whr.eq("id",id);
        }
        whr
    }

    pub fn query_prepare(&self,param:&ServiceParam) -> (String,Vec<serde_json::Value>) {
        B::prepare(
            B::new_sql("select id, namespace_id, service_name, group_name, instance_size, healthy_size, threshold, metadata, extend_info, create_time, last_time from tb_service")
            .push_build(&mut self.conditions(param))
        )
    }

    pub fn query_count_prepare(&self,param:&ServiceParam) -> (String,Vec<serde_json::Value>) {
        B::prepare(
            B::new_sql("select count(1) from tb_service")
            .push_build(&mut self.conditions(param))
        )
    }

    pub fn insert_prepare(&self,record:&ServiceDO) -> (String,Vec<serde_json::Value>) {
        let mut field_builder=B::new_comma_paren();
        let mut value_builder=B::new_comma_paren();
        if let Some(id) = &record.id {
            field_builder.push_sql("id");
            value_builder.push("?",id);
        }
        if let Some(namespace_id) = &record.namespace_id {
            field_builder.push_sql("namespace_id");
            value_builder.push("?",namespace_id);
        }
        if let Some(service_name) = &record.service_name {
            field_builder.push_sql("service_name");
            value_builder.push("?",service_name);
        }
        if let Some(group_name) = &record.group_name {
            field_builder.push_sql("group_name");
            value_builder.push("?",group_name);
        }
        if let Some(instance_size) = &record.instance_size {
            field_builder.push_sql("instance_size");
            value_builder.push("?",instance_size);
        }
        if let Some(healthy_size) = &record.healthy_size {
            field_builder.push_sql("healthy_size");
            value_builder.push("?",healthy_size);
        }
        if let Some(threshold) = &record.threshold {
            field_builder.push_sql("threshold");
            value_builder.push("?",threshold);
        }
        if let Some(metadata) = &record.metadata {
            field_builder.push_sql("metadata");
            value_builder.push("?",metadata);
        }
        if let Some(extend_info) = &record.extend_info {
            field_builder.push_sql("extend_info");
            value_builder.push("?",extend_info);
        }
        if let Some(create_time) = &record.create_time {
            field_builder.push_sql("create_time");
            value_builder.push("?",create_time);
        }
        if let Some(last_time) = &record.last_time {
            field_builder.push_sql("last_time");
            value_builder.push("?",last_time);
        }
        B::prepare(
            B::new_sql("insert into tb_service")
            .push_build(&mut field_builder)
            .push_sql("values")
            .push_build(&mut value_builder)
        )
    }

    pub fn update_prepare(&self,record:&ServiceDO,param:&ServiceParam) -> (String,Vec<serde_json::Value>) {
        let mut set_builder=B::new_comma();
        if let Some(id) = &record.id {
            set_builder.eq("id",id);
        }
        if let Some(namespace_id) = &record.namespace_id {
            set_builder.eq("namespace_id",namespace_id);
        }
        if let Some(service_name) = &record.service_name {
            set_builder.eq("service_name",service_name);
        }
        if let Some(group_name) = &record.group_name {
            set_builder.eq("group_name",group_name);
        }
        if let Some(instance_size) = &record.instance_size {
            set_builder.eq("instance_size",instance_size);
        }
        if let Some(healthy_size) = &record.healthy_size {
            set_builder.eq("healthy_size",healthy_size);
        }
        if let Some(threshold) = &record.threshold {
            set_builder.eq("threshold",threshold);
        }
        if let Some(metadata) = &record.metadata {
            set_builder.eq("metadata",metadata);
        }
        if let Some(extend_info) = &record.extend_info {
            set_builder.eq("extend_info",extend_info);
        }
        if let Some(create_time) = &record.create_time {
            set_builder.eq("create_time",create_time);
        }
        if let Some(last_time) = &record.last_time {
            set_builder.eq("last_time",last_time);
        }
        let mut whr = self.conditions(param);
        if whr.is_empty() {
            panic!("update conditions is empty");
        }
        B::prepare(
             B::new_sql("update tb_service set ")
            .push_build(&mut set_builder)
            .push_build(&mut whr)
        )
    }

    pub fn delete_prepare(&self,param:&ServiceParam) -> (String,Vec<serde_json::Value>) {
        B::prepare(
            B::new_sql("delete from tb_service")
            .push_build(&mut self.conditions(param))
        )
    }
}

pub struct ServiceDao{
    conn: Arc<Connection>,
    inner: ServiceSql,
}

impl ServiceDao {
    pub fn new(conn: Arc<Connection>) -> Self{
        Self{ 
            conn,
            inner:ServiceSql,
        }
    }

    pub fn execute(&self,sql:&str,args:&Vec<serde_json::Value>) -> Result<usize,String>{
        sqlite_execute(&self.conn,sql,args) 
    }

    pub fn fetch(&self,sql:&str,args:&Vec<serde_json::Value>) -> Result<Vec<ServiceDO>,String> {
        sqlite_fetch(&self.conn,sql,args,ServiceDO::from_row)
    }

    pub fn fetch_count(&self,sql:&str,args:&Vec<serde_json::Value>) -> Result<u64,String> {
        sqlite_fetch_count(&self.conn,sql,args)
    }

    pub fn insert(&self,record:&ServiceDO) -> Result<usize,String> {
        let (sql,args) = self.inner.insert_prepare(record);
        self.execute(&sql, &args)
    }

    pub fn update(&self,record:&ServiceDO,param:&ServiceParam) -> Result<usize,String> {
        let (sql,args) = self.inner.update_prepare(record,param);
        self.execute(&sql, &args)
    }

    pub fn delete(&self,param:&ServiceParam) -> Result<usize,String> {
        let (sql,args) = self.inner.delete_prepare(param);
        self.execute(&sql, &args)
    }

    pub fn query(&self,param:&ServiceParam) -> Result<Vec<ServiceDO>,String> {
        let (sql,args) = self.inner.query_prepare(param);
        self.fetch(&sql, &args)
    }

    pub fn query_count(&self,param:&ServiceParam) -> Result<u64,String> {
        let (sql,args) = self.inner.query_count_prepare(param);
        self.fetch_count(&sql, &args)
    }

}