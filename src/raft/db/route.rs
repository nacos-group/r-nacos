use std::sync::Arc;

use actix::prelude::*;

use crate::{
    grpc::PayloadUtils,
    raft::{
        cluster::{
            model::{RouteAddr, RouterRequest},
            route::RaftAddrRouter,
        },
        network::factory::RaftClusterRequestSender,
    },
};

use super::table::{TableManager, TableManagerAsyncReq, TableManagerQueryReq, TableManagerReq};

pub struct TableRoute {
    table_manage: Addr<TableManager>,
    raft_addr_route: Arc<RaftAddrRouter>,
    cluster_sender: Arc<RaftClusterRequestSender>,
}

impl TableRoute {
    pub fn new(
        table_manage: Addr<TableManager>,
        raft_addr_route: Arc<RaftAddrRouter>,
        cluster_sender: Arc<RaftClusterRequestSender>,
    ) -> Self {
        Self {
            table_manage,
            raft_addr_route,
            cluster_sender,
        }
    }

    fn unknown_err(&self) -> anyhow::Error {
        anyhow::anyhow!("unknown the raft leader addr!")
    }

    pub async fn request(&self, cmd: TableManagerReq) -> anyhow::Result<()> {
        match self.raft_addr_route.get_route_addr().await? {
            RouteAddr::Local => {
                self.table_manage
                    .send(TableManagerAsyncReq(cmd))
                    .await?
                    .ok();
            }
            RouteAddr::Remote(_, addr) => {
                let req: RouterRequest = cmd.into();
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

    pub async fn get_leader_data(&self, cmd: TableManagerQueryReq) -> anyhow::Result<()> {
        Ok(())
    }
}
