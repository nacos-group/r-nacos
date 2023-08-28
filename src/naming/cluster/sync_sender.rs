
use std::{sync::Arc, time::Duration};

use actix::prelude::*;

use crate::{raft::network::factory::RaftClusterRequestSender, grpc::PayloadUtils};

use super::model::{SyncSenderRequest, SyncSenderResponse, NamingRouteRequest, NamingRouterResponse, SyncSenderSetCmd};

pub struct ClusteSyncSender {
    target_id:u64,
    target_addr:Arc<String>,
    cluster_sender: Arc<RaftClusterRequestSender>,
}

impl ClusteSyncSender {
    pub fn new(target_id:u64,
            target_addr:Arc<String>,
            cluster_sender: Arc<RaftClusterRequestSender>) -> Self {
        Self {
            target_id,
            target_addr,
            cluster_sender,
        }
    }
}

impl Actor for ClusteSyncSender {
    type Context = Context<Self>;

    fn stopped(&mut self, ctx: &mut Self::Context) {
        log::info!("ClusteSyncSender started,target_id:{}",&self.target_id);
    }
}

impl Handler<SyncSenderSetCmd> for ClusteSyncSender {
    type Result = anyhow::Result<SyncSenderResponse>;

    fn handle(&mut self, msg: SyncSenderSetCmd, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            SyncSenderSetCmd::UpdateTargetAddr(target_addr) => {
                self.target_addr = target_addr;
                Ok(SyncSenderResponse::None)
            },
        }
    }
}

impl Handler<SyncSenderRequest> for ClusteSyncSender {
    type Result = ResponseActFuture<Self, anyhow::Result<SyncSenderResponse>>;

    fn handle(&mut self, msg: SyncSenderRequest, _ctx: &mut Self::Context) -> Self::Result {
        let cluster_sender = self.cluster_sender.clone();
        let target_addr = self.target_addr.clone();
        let fut = async move {
            let req = msg.0;
            let request = serde_json::to_string(&req).unwrap_or_default();
            let payload = PayloadUtils::build_payload("NamingRouteRequest", request);
            let resp_payload = match cluster_sender.send_request(target_addr.clone(), payload).await {
                Ok(v) => v,
                Err(_) => {
                    //retry request
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    let request = serde_json::to_string(&req).unwrap_or_default();
                    let payload = PayloadUtils::build_payload("NamingRouteRequest", request);
                    cluster_sender.send_request(target_addr, payload).await?
                },
            };
            let body_vec = resp_payload.body.unwrap_or_default().value;
            let _: NamingRouterResponse = serde_json::from_slice(&body_vec)?;
            Ok(SyncSenderResponse::None)
        }
        .into_actor(self)
        .map(|r, _act, _ctx| r);
        Box::pin(fut)
    }
}
