use std::{collections::HashMap, sync::Arc, fmt::Debug};

use actix::prelude::*;
use async_raft::RaftStorage;
use serde::{Serialize, Deserialize};

use crate::{config::core::{ConfigActor, ConfigKey}, raft::asyncraft::store::store::AStore};


#[derive(Clone)]
pub struct RaftAddrRouter{
    raft_store: Arc<AStore>,
}

impl Debug for RaftAddrRouter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AddrRouter").finish()
    }
}

impl RaftAddrRouter {
    pub async fn get_route_addr(&self) -> anyhow::Result<RouteAddr> {
        let member = self.raft_store.get_membership_config().await?;
        Ok(RouteAddr::Unknown)
    }
}


#[derive(Clone,Debug)]
pub struct ConfigProxy{
    addr: Addr<ConfigActor>,
}

impl ConfigProxy {
    pub fn new(addr:Addr<ConfigActor>) -> Self {
        Self {
            addr,
        }
    }

    pub fn write_addr(&self) -> RouteAddr {
        RouteAddr::Unknown
    }

    pub async fn set_config(&self,req:SetConfigReq) -> anyhow::Result<()> {
        Ok(())
    }

    pub async fn del_config(&self,req:DelConfigReq) -> anyhow::Result<()> {
        Ok(())
    }
}


pub enum RouteAddr {
    Local,
    Remote(u64,Arc<String>),
    Unknown,
}

#[derive(Clone, Debug)]
pub struct SetConfigReq {
    pub config_key: ConfigKey,
    pub value: Arc<String>, 
    pub is_local: bool,
    //pub extend_info: Option<HashMap<String,String>>,
}

#[derive(Clone, Debug)]
pub struct DelConfigReq {
    pub config_key: ConfigKey,
    pub is_local: bool,
    //pub extend_info: Option<HashMap<String,String>>,
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RouterRequest {
    ConfigSet {
        key: String,
        value: String,
        extend_info: HashMap<String,String>,
    },
    ConfigDel {
        key: String,
        extend_info: HashMap<String,String>,
    },
}