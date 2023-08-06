use std::{fmt::Debug, sync::Arc};

use actix::prelude::*;

use crate::{
    config::core::{ConfigActor, ConfigAsyncCmd},
    grpc::PayloadUtils,
    raft::{
        NacosRaft,
        {network::factory::RaftClusterRequestSender, store::core::RaftStore},
    },
};

use super::model::{DelConfigReq, RouteAddr, RouterRequest, RouterResponse, SetConfigReq};

#[derive(Clone)]
pub struct RaftAddrRouter {
    raft_store: Arc<RaftStore>,
    raft: Arc<NacosRaft>,
    local_node_id: u64,
}

impl Debug for RaftAddrRouter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AddrRouter").finish()
    }
}

impl RaftAddrRouter {
    pub fn new(raft: Arc<NacosRaft>, raft_store: Arc<RaftStore>, local_node_id: u64) -> Self {
        Self {
            raft,
            raft_store,
            local_node_id,
        }
    }

    pub async fn get_route_addr(&self) -> anyhow::Result<RouteAddr> {
        //let state = self.raft_store.get_initial_state().await?;
        let leader = self.raft.current_leader().await;
        match leader {
            Some(node_id) => {
                if node_id == self.local_node_id {
                    Ok(RouteAddr::Local)
                } else {
                    let addr = self.raft_store.get_target_addr(node_id).await?;
                    Ok(RouteAddr::Remote(node_id, addr))
                }
            }
            None => Ok(RouteAddr::Unknown),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ConfigRoute {
    config_addr: Addr<ConfigActor>,
    raft_addr_route: Arc<RaftAddrRouter>,
    cluster_sender: Arc<RaftClusterRequestSender>,
}

impl ConfigRoute {
    pub fn new(
        config_addr: Addr<ConfigActor>,
        raft_addr_route: Arc<RaftAddrRouter>,
        cluster_sender: Arc<RaftClusterRequestSender>,
    ) -> Self {
        Self {
            config_addr,
            raft_addr_route,
            cluster_sender,
        }
    }

    fn unknown_err(&self) -> anyhow::Error {
        anyhow::anyhow!("unknown the raft leader addr!")
    }

    pub async fn set_config(&self, req: SetConfigReq) -> anyhow::Result<()> {
        match self.raft_addr_route.get_route_addr().await? {
            RouteAddr::Local => {
                let cmd = ConfigAsyncCmd::Add(req.config_key, req.value);
                self.config_addr.send(cmd).await?.ok();
            }
            RouteAddr::Remote(_, addr) => {
                let req: RouterRequest = req.into();
                let request = serde_json::to_string(&req).unwrap_or_default();
                let payload = PayloadUtils::build_payload("RaftRouteRequest", request);
                let resp_payload = self.cluster_sender.send_request(addr, payload).await?;
                let body_vec = resp_payload.body.unwrap_or_default().value;
                let _: RouterResponse = serde_json::from_slice(&body_vec)?;
            }
            RouteAddr::Unknown => {
                return Err(self.unknown_err());
            }
        }
        Ok(())
    }

    pub async fn del_config(&self, req: DelConfigReq) -> anyhow::Result<()> {
        match self.raft_addr_route.get_route_addr().await? {
            RouteAddr::Local => {
                let cmd = ConfigAsyncCmd::Delete(req.config_key);
                self.config_addr.send(cmd).await?.ok();
            }
            RouteAddr::Remote(_, addr) => {
                let req: RouterRequest = req.into();
                let request = serde_json::to_string(&req).unwrap_or_default();
                let payload = PayloadUtils::build_payload("RaftRouteRequest", request);
                let resp_payload = self.cluster_sender.send_request(addr, payload).await?;
                let body_vec = resp_payload.body.unwrap_or_default().value;
                let _: RouterResponse = serde_json::from_slice(&body_vec)?;
            }
            RouteAddr::Unknown => {
                return Err(self.unknown_err());
            }
        }
        Ok(())
    }
}
