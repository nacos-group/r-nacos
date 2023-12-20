use std::sync::Arc;

use actix::prelude::*;

use crate::{
    grpc::PayloadUtils,
    raft::{
        cluster::{
            model::{RouteAddr, RouterRequest, RouterResponse},
            route::RaftAddrRouter,
        },
        network::factory::RaftClusterRequestSender,
    },
};

use super::table::{
    TableManager, TableManagerAsyncReq, TableManagerQueryReq, TableManagerReq, TableManagerResult,
};

pub struct TableRoute {
    table_manager: Addr<TableManager>,
    raft_addr_route: Arc<RaftAddrRouter>,
    cluster_sender: Arc<RaftClusterRequestSender>,
}

impl TableRoute {
    pub fn new(
        table_manager: Addr<TableManager>,
        raft_addr_route: Arc<RaftAddrRouter>,
        cluster_sender: Arc<RaftClusterRequestSender>,
    ) -> Self {
        Self {
            table_manager,
            raft_addr_route,
            cluster_sender,
        }
    }

    fn unknown_err(&self) -> anyhow::Error {
        anyhow::anyhow!("unknown the raft leader addr!")
    }

    pub async fn request(&self, req: TableManagerReq) -> anyhow::Result<()> {
        match self.raft_addr_route.get_route_addr().await? {
            RouteAddr::Local => {
                self.table_manager
                    .send(TableManagerAsyncReq(req))
                    .await?
                    .ok();
            }
            RouteAddr::Remote(_, addr) => {
                let req: RouterRequest = req.into();
                let request = serde_json::to_string(&req).unwrap_or_default();
                let payload = PayloadUtils::build_payload("RaftRouteRequest", request);
                let _resp_payload = self.cluster_sender.send_request(addr, payload).await?;
            }
            RouteAddr::Unknown => {
                return Err(self.unknown_err());
            }
        };
        Ok(())
    }

    pub async fn get_leader_data(
        &self,
        req: TableManagerQueryReq,
    ) -> anyhow::Result<TableManagerResult> {
        match self.raft_addr_route.get_route_addr().await? {
            RouteAddr::Local => self.table_manager.send(req).await?,
            RouteAddr::Remote(_, addr) => {
                let req: RouterRequest = req.into();
                let request = serde_json::to_string(&req).unwrap_or_default();
                let payload = PayloadUtils::build_payload("RaftRouteRequest", request);
                let resp_payload = self.cluster_sender.send_request(addr, payload).await?;
                let body_vec = resp_payload.body.unwrap_or_default().value;
                let resp: RouterResponse = serde_json::from_slice(&body_vec)?;
                match resp {
                    RouterResponse::TableManagerResult { result } => Ok(result),
                    _ => Err(anyhow::anyhow!(
                        "TableManagerQueryReq response type is error!"
                    )),
                }
            }
            RouteAddr::Unknown => Err(self.unknown_err()),
        }
    }
}
