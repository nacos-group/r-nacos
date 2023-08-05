use std::{collections::HashMap, sync::Arc, fmt::Debug, time::Duration};

use actix::prelude::*;
use serde::{Serialize, Deserialize};

use crate::{config::core::{ConfigActor, ConfigKey, ConfigAsyncCmd}, raft::{asyncraft::store::store::AStore, NacosRaft}};


#[derive(Clone)]
pub struct RaftAddrRouter{
    raft_store: Arc<AStore>,
    raft: Arc<NacosRaft>,
    local_node_id: u64,
}

impl Debug for RaftAddrRouter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AddrRouter").finish()
    }
}

impl RaftAddrRouter {

    pub fn new(raft: Arc<NacosRaft>,raft_store: Arc<AStore>,local_node_id: u64) -> Self {
        Self {
            raft,
            raft_store,
            local_node_id
        }
    }

    pub async fn get_route_addr(&self) -> anyhow::Result<RouteAddr> {
        //let state = self.raft_store.get_initial_state().await?;
        let leader = self.raft.current_leader().await;
        match leader {
            Some(node_id) => {
                if node_id == self.local_node_id {
                    Ok(RouteAddr::Local)
                }
                else{
                    let addr = self.raft_store.get_target_addr(node_id).await?;
                    Ok(RouteAddr::Remote(node_id, addr))
                }
            },
            None => Ok(RouteAddr::Unknown),
        }
    }
}


#[derive(Clone,Debug)]
pub struct ConfigRoute{
    config_addr: Addr<ConfigActor>,
    raft_addr_route: Arc<RaftAddrRouter>,
    client: reqwest::Client,
}

impl ConfigRoute {
    pub fn new(config_addr:Addr<ConfigActor>,raft_addr_route: Arc<RaftAddrRouter>) -> Self {
        let client = reqwest::ClientBuilder::new().build().unwrap();
        Self {
            config_addr,
            raft_addr_route,
            client,
        }
    }

    fn unknown_err(&self) -> anyhow::Error {
        anyhow::anyhow!("unknown the raft leader addr!")
    }

    pub async fn set_config(&self,req:SetConfigReq) -> anyhow::Result<()> {
        match self.raft_addr_route.get_route_addr().await? {
            RouteAddr::Local => {
                let cmd=ConfigAsyncCmd::Add(req.config_key,req.value);
                self.config_addr.send(cmd).await?.ok();
            },
            RouteAddr::Remote(_, addr) => {
                let url = format!("http://{}/nacos/v1/raft/route", &addr);
                let req:RouterRequest  = req.into();
                let resp = self.client.post(url)
                .timeout(Duration::from_millis(3000))
                .json(&req)
                .send()
                .await?;
                let _:RouterResponse = resp.json().await?;
            },
            RouteAddr::Unknown => {
                return Err(self.unknown_err());
            },
        }
        Ok(())
    }

    pub async fn del_config(&self,req:DelConfigReq) -> anyhow::Result<()> {
        match self.raft_addr_route.get_route_addr().await? {
            RouteAddr::Local => {
                let cmd=ConfigAsyncCmd::Delete(req.config_key);
                self.config_addr.send(cmd).await?.ok();
            },
            RouteAddr::Remote(_, addr) => {
                let url = format!("http://{}/nacos/v1/raft/route", &addr);
                let req:RouterRequest  = req.into();
                let resp = self.client.post(url)
                .json(&req)
                .send()
                .await?;
                let _:RouterResponse = resp.json().await?;
            },
            RouteAddr::Unknown => {
                return Err(self.unknown_err());
            },
        }
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
    //pub can_route_to_remote: bool,
    //pub extend_info: Option<HashMap<String,String>>,
}

impl SetConfigReq {
    pub fn new(config_key:ConfigKey,value:Arc<String>) -> Self {
        Self {
            config_key,
            value,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DelConfigReq {
    pub config_key: ConfigKey,
    //pub can_route_to_remote: bool,
    //pub extend_info: Option<HashMap<String,String>>,
}

impl DelConfigReq {
    pub fn new(config_key: ConfigKey) -> Self {
        Self {
            config_key
        }
    }
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RouterRequest {
    ConfigSet {
        key: String,
        value: Arc<String>,
        extend_info: HashMap<String,String>,
    },
    ConfigDel {
        key: String,
        extend_info: HashMap<String,String>,
    },
}

impl From<SetConfigReq> for RouterRequest {
    fn from(req: SetConfigReq) -> Self {
        Self::ConfigSet { 
            key:req.config_key.build_key(), 
            value: req.value, 
            extend_info: Default::default() 
        }
    }
}

impl From<DelConfigReq> for RouterRequest {
    fn from(req: DelConfigReq) -> Self {
        Self::ConfigDel {
            key:req.config_key.build_key(), 
            extend_info: Default::default() 
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RouterResponse {
    None,
}