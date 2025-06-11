use std::sync::Arc;

use rusqlite::Connection;

use crate::{now_millis, now_millis_i64};

use super::{service_dao::ServiceDao, service_do::{ServiceDO, ServiceParam}};

use actix::prelude::*;

pub struct ServiceDalActor {
    pub(crate) service_dao: ServiceDao,
}

impl ServiceDalActor {
    pub fn new() -> Self {
        let conn = Connection::open_in_memory().unwrap();
        //let naming_db = std::env::var("naming.db").unwrap_or("naming.db".to_owned());
        //let conn = Connection::open(&naming_db).unwrap();
        Self::init_table(&conn);
        Self { service_dao: ServiceDao::new(Arc::new(conn))}
    }
    pub fn new_by_conn(conn: Arc<Connection>) -> Self {
        Self::init_table(&conn);
        Self { service_dao: ServiceDao::new(conn)}
    }

    fn init_table(conn:&Connection){
        let create_table_sql = r"
create table if not exists tb_service(
    id integer primary key autoincrement,
    namespace_id varchar(255),
    service_name varchar(255),
    group_name varchar(255),
    instance_size integer,
    healthy_size integer,
    threshold float,
    metadata text,
    extend_info text,
    create_time long,
    last_time long
);
create index if not exists tb_service_key_idx on tb_service(namespace_id,service_name,group_name);
        ";
        conn.execute_batch(create_table_sql).unwrap();
    }

    pub fn update_service(&self,mut service:ServiceDO) -> anyhow::Result<()> {
        let now = now_millis_i64();
        service.last_time = Some(now);
        if let Some(param) =service.get_key_param() {
            let c = match self.service_dao.update(&service, &param) {
                Ok(c) => {c},
                Err(_) => {0} ,
            };
            if c==0 {
                service.create_time = Some(now);
                self.service_dao.insert(&service)?;
            }
            Ok(())
        }
        else{
            Err(anyhow::anyhow!("update_service:invalid service info"))
        }
    }
}

impl Actor for ServiceDalActor {
    type Context = SyncContext<Self>;
}

#[derive(Debug,Message)]
#[rtype(result = "anyhow::Result<ServiceDalResult>")]
pub enum ServiceDalMsg {
    AddService(ServiceDO),
    UpdateService(ServiceDO),
    DeleteService(ServiceParam),
    QueryServiceCount(ServiceParam),
    QueryServiceList(ServiceParam),
    QueryServiceListWithCount(ServiceParam),
}

pub enum ServiceDalResult {
    None,
    Count(u64),
    ServiceList(Vec<ServiceDO>),
    ServiceListWithCount(Vec<ServiceDO>,u64),
}

impl Handler<ServiceDalMsg> for ServiceDalActor {
    type Result = anyhow::Result<ServiceDalResult>;

    fn handle(&mut self, msg: ServiceDalMsg, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            ServiceDalMsg::AddService(service) => {
                self.update_service(service)?;
            },
            ServiceDalMsg::UpdateService(service) => {
                self.update_service(service)?;
            },
            ServiceDalMsg::DeleteService(param) => {
                self.service_dao.delete(&param)?;
            },
            ServiceDalMsg::QueryServiceCount(param) => {
                let count = self.service_dao.query_count(&param)?;
                return Ok(ServiceDalResult::Count(count));
            },
            ServiceDalMsg::QueryServiceList(param) => {
                let list = self.service_dao.query(&param)?;
                return Ok(ServiceDalResult::ServiceList(list));
            },
            ServiceDalMsg::QueryServiceListWithCount(param) => {
                let count = self.service_dao.query_count(&param)?;
                let list = self.service_dao.query(&param)?;
                println!("QueryServiceListWithCount,{},{}",&count,list.len());
                return Ok(ServiceDalResult::ServiceListWithCount(list,count));
            },
        }
        Ok(ServiceDalResult::None)
    }
}