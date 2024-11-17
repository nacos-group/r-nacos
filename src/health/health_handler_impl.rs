use crate::config::core::ConfigActor;
use crate::health::model::{HealthBackRequest, HealthCheckRequest, HealthCheckType};
use crate::naming::core::NamingActor;
use crate::raft::cache::CacheManager;
use crate::raft::filestore::raftapply::StateApplyManager;
use crate::raft::filestore::raftindex::RaftIndexManager;
use crate::raft::filestore::raftlog::RaftLogManager;
use crate::user::UserManager;
use actix::Handler;

impl Handler<HealthCheckRequest> for ConfigActor {
    type Result = anyhow::Result<()>;

    fn handle(&mut self, msg: HealthCheckRequest, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            HealthCheckRequest::Ping(addr) => {
                addr.do_send(HealthBackRequest::Pong(HealthCheckType::Config))
            }
        }
        Ok(())
    }
}

impl Handler<HealthCheckRequest> for NamingActor {
    type Result = anyhow::Result<()>;

    fn handle(&mut self, msg: HealthCheckRequest, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            HealthCheckRequest::Ping(addr) => {
                addr.do_send(HealthBackRequest::Pong(HealthCheckType::Naming))
            }
        }
        Ok(())
    }
}

impl Handler<HealthCheckRequest> for UserManager {
    type Result = anyhow::Result<()>;

    fn handle(&mut self, msg: HealthCheckRequest, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            HealthCheckRequest::Ping(addr) => {
                addr.do_send(HealthBackRequest::Pong(HealthCheckType::User))
            }
        }
        Ok(())
    }
}

impl Handler<HealthCheckRequest> for CacheManager {
    type Result = anyhow::Result<()>;

    fn handle(&mut self, msg: HealthCheckRequest, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            HealthCheckRequest::Ping(addr) => {
                addr.do_send(HealthBackRequest::Pong(HealthCheckType::Cache))
            }
        }
        Ok(())
    }
}

impl Handler<HealthCheckRequest> for RaftIndexManager {
    type Result = anyhow::Result<()>;

    fn handle(&mut self, msg: HealthCheckRequest, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            HealthCheckRequest::Ping(addr) => {
                addr.do_send(HealthBackRequest::Pong(HealthCheckType::RaftStoreIndex))
            }
        }
        Ok(())
    }
}
impl Handler<HealthCheckRequest> for RaftLogManager {
    type Result = anyhow::Result<()>;

    fn handle(&mut self, msg: HealthCheckRequest, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            HealthCheckRequest::Ping(addr) => {
                addr.do_send(HealthBackRequest::Pong(HealthCheckType::RaftStoreLog))
            }
        }
        Ok(())
    }
}
impl Handler<HealthCheckRequest> for StateApplyManager {
    type Result = anyhow::Result<()>;

    fn handle(&mut self, msg: HealthCheckRequest, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            HealthCheckRequest::Ping(addr) => {
                addr.do_send(HealthBackRequest::Pong(HealthCheckType::RaftStoreApply))
            }
        }
        Ok(())
    }
}
