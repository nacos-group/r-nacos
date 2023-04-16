use std::sync::Arc;

use rusqlite::Connection;

use super::{service_dao::ServiceDao, service_do::ServiceDO};

use actix::prelude::*;

pub(crate) struct ServiceDalActor {
    pub service_dao: ServiceDao,
}

impl ServiceDalActor {
    pub fn new() -> Self {
        let conn = Connection::open_in_memory().unwrap();
        Self { service_dao: ServiceDao::new(Arc::new(conn))}
    }
    pub fn new_by_conn(conn: Arc<Connection>) -> Self {
        Self { service_dao: ServiceDao::new(conn)}
    }

    pub fn add_service(&mut self,service:ServiceDO) {
        //self.service_dao
    }
}

impl Actor for ServiceDalActor {
    type Context = SyncContext<Self>;
}