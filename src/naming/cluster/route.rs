use std::sync::Arc;

use crate::{naming::{core::{NamingActor, NamingCmd, NamingResult}, model::{Instance, InstanceUpdateTag}}, raft::network::factory::RaftClusterRequestSender, grpc::PayloadUtils};

use super::{node_manage::NodeManage, model::{NamingRouteAddr, NamingRouterRequest, NamingRouterResponse}};

use actix::prelude::*;

#[derive(Clone, Debug)]
pub struct NamingRoute {
    naming_addr: Addr<NamingActor>,
    node_manage: Arc<NodeManage>,
    cluster_sender: Arc<RaftClusterRequestSender>,
}

impl NamingRoute{
    pub fn new(naming_addr: Addr<NamingActor>,
        node_manage: Arc<NodeManage>,
        cluster_sender: Arc<RaftClusterRequestSender>) -> Self {
        Self {
            naming_addr,
            node_manage,
            cluster_sender,
        }
    }

    pub async fn update_instance(&self,instance: Instance,tag: Option<InstanceUpdateTag>) -> anyhow::Result<()> {
        let key = instance.get_service_key();
        match self.node_manage.route_addr(&key).await {
            NamingRouteAddr::Local(_) => {
                let cmd = NamingCmd::Update(instance, tag);
                let _:NamingResult  = self.naming_addr.send(cmd).await??;
            },
            NamingRouteAddr::Remote(_, addr) => {
                let req = NamingRouterRequest::UpdateInstance { instance, tag };
                let request = serde_json::to_string(&req).unwrap_or_default();
                let payload = PayloadUtils::build_payload("NamingRouterRequest", request);
                let resp_payload = self.cluster_sender.send_request(addr, payload).await?;
                let body_vec = resp_payload.body.unwrap_or_default().value;
                let _: NamingRouterResponse = serde_json::from_slice(&body_vec)?;
            },
        };
        Ok(())
    }

    pub async fn delete_instance(&self,instance: Instance) -> anyhow::Result<()> {
        let key = instance.get_service_key();
        match self.node_manage.route_addr(&key).await {
            NamingRouteAddr::Local(_) => {
                let cmd = NamingCmd::Delete(instance);
                let _:NamingResult  = self.naming_addr.send(cmd).await??;
            },
            NamingRouteAddr::Remote(_, addr) => {
                let req = NamingRouterRequest::RemoveInstance { instance };
                let request = serde_json::to_string(&req).unwrap_or_default();
                let payload = PayloadUtils::build_payload("NamingRouterRequest", request);
                let resp_payload = self.cluster_sender.send_request(addr, payload).await?;
                let body_vec = resp_payload.body.unwrap_or_default().value;
                let _: NamingRouterResponse = serde_json::from_slice(&body_vec)?;
            }
        };
        Ok(())
    }

}