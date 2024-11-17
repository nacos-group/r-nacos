use crate::health::core::HealthManager;
use actix::{Addr, Message};

#[derive(Debug, Clone)]
pub enum CheckHealthResult {
    Success,
    //Warning(String),
    Error(String),
}

impl CheckHealthResult {
    pub fn is_success(&self) -> bool {
        match self {
            CheckHealthResult::Success => true,
            CheckHealthResult::Error(_) => false,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum HealthCheckType {
    Config,
    Naming,
    User,
    Cache,
    RaftStoreIndex,
    RaftStoreLog,
    RaftStoreApply,
    RaftCluster,
}

impl HealthCheckType {
    pub fn name(&self) -> &'static str {
        match self {
            HealthCheckType::Config => "Config",
            HealthCheckType::Naming => "Naming",
            HealthCheckType::User => "User",
            HealthCheckType::Cache => "Cache",
            HealthCheckType::RaftStoreIndex => "RaftStoreIndex",
            HealthCheckType::RaftStoreLog => "RaftStoreLog",
            HealthCheckType::RaftStoreApply => "RaftStoreApply",
            HealthCheckType::RaftCluster => "RaftCluster",
        }
    }
}

#[derive(Message, Debug, Clone)]
#[rtype(result = "anyhow::Result<()>")]
pub enum HealthCheckRequest {
    Ping(Addr<HealthManager>),
}

#[derive(Message, Debug, Clone)]
#[rtype(result = "anyhow::Result<()>")]
pub enum HealthBackRequest {
    Pong(HealthCheckType),
}

#[derive(Message, Debug, Clone)]
#[rtype(result = "anyhow::Result<HealthManagerResponse>")]
pub enum HealthManagerRequest {
    Status,
}

#[derive(Debug, Clone)]
pub enum HealthManagerResponse {
    StatusResult(CheckHealthResult),
}

#[derive(Debug, Clone)]
pub struct HealthCheckItem {
    pub last_success_time: u64,
    pub timeout: u64,
    pub check_type: HealthCheckType,
}

impl HealthCheckItem {
    pub fn new(
        check_type: HealthCheckType,
        timeout: u64,
        last_success_time: u64,
    ) -> HealthCheckItem {
        Self {
            check_type,
            timeout,
            last_success_time,
        }
    }

    pub fn check(&self, now: u64) -> CheckHealthResult {
        if now > self.last_success_time + self.timeout {
            CheckHealthResult::Error(format!("{} module ill.", self.check_type.name()))
        } else {
            CheckHealthResult::Success
        }
    }
}
