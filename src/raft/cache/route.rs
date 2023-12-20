use std::sync::Arc;

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

use super::{CacheLimiterReq, CacheManager, CacheManagerResult};
use actix::prelude::*;

pub struct CacheRoute {
    cache_manager: Addr<CacheManager>,
    raft_addr_route: Arc<RaftAddrRouter>,
    cluster_sender: Arc<RaftClusterRequestSender>,
}

impl CacheRoute {
    pub fn new(
        cache_manager: Addr<CacheManager>,
        raft_addr_route: Arc<RaftAddrRouter>,
        cluster_sender: Arc<RaftClusterRequestSender>,
    ) -> Self {
        Self {
            cache_manager,
            raft_addr_route,
            cluster_sender,
        }
    }

    fn unknown_err(&self) -> anyhow::Error {
        anyhow::anyhow!("unknown the raft leader addr!")
    }

    pub async fn request_limiter(
        &self,
        req: CacheLimiterReq,
    ) -> anyhow::Result<CacheManagerResult> {
        match self.raft_addr_route.get_route_addr().await? {
            RouteAddr::Local => self.cache_manager.send(req).await?,
            RouteAddr::Remote(_, addr) => {
                let req: RouterRequest = req.into();
                let request = serde_json::to_string(&req).unwrap_or_default();
                let payload = PayloadUtils::build_payload("RaftRouteRequest", request);
                let resp_payload = self.cluster_sender.send_request(addr, payload).await?;
                let body_vec = resp_payload.body.unwrap_or_default().value;
                let resp: RouterResponse = serde_json::from_slice(&body_vec)?;
                match resp {
                    RouterResponse::CacheManagerResult { result } => Ok(result),
                    _ => Err(anyhow::anyhow!("response type is error!")),
                }
            }
            RouteAddr::Unknown => Err(self.unknown_err()),
        }
    }
}
