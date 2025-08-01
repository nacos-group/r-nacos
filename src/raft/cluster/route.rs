use super::model::{DelConfigReq, RouteAddr, RouterRequest, RouterResponse, SetConfigReq};
use crate::grpc::handler::RAFT_ROUTE_REQUEST;
use crate::namespace::model::{NamespaceRaftReq, NamespaceRaftResult};
use crate::raft::cluster::router_request;
use crate::raft::filestore::core::FileStore;
use crate::raft::store::{ClientRequest, ClientResponse};
use crate::transfer::model::{TransferImportParam, TransferImportRequest, TransferImportResponse};
use crate::transfer::reader::TransferImportManager;
use crate::{
    config::core::{ConfigActor, ConfigAsyncCmd, ConfigCmd},
    grpc::PayloadUtils,
    raft::{network::factory::RaftClusterRequestSender, NacosRaft},
};
use actix::prelude::*;
use async_raft_ext::raft::ClientWriteRequest;
use std::convert::TryInto;
use std::{fmt::Debug, sync::Arc};

#[derive(Clone)]
pub struct RaftAddrRouter {
    raft_store: Arc<FileStore>,
    raft: Arc<NacosRaft>,
    local_node_id: u64,
}

impl Debug for RaftAddrRouter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AddrRouter").finish()
    }
}

impl RaftAddrRouter {
    pub fn new(raft: Arc<NacosRaft>, raft_store: Arc<FileStore>, local_node_id: u64) -> Self {
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
                let cmd = ConfigAsyncCmd::Add {
                    key: req.config_key,
                    value: req.value,
                    op_user: req.op_user,
                    config_type: req.config_type,
                    desc: req.desc,
                };
                self.config_addr.send(cmd).await?.ok();
            }
            RouteAddr::Remote(_, addr) => {
                let source_req = req.clone();
                let req: RouterRequest = req.into();
                let request = serde_json::to_string(&req).unwrap_or_default();
                let payload = PayloadUtils::build_payload(RAFT_ROUTE_REQUEST, request);
                let resp_payload = self.cluster_sender.send_request(addr, payload).await?;
                let body_vec = resp_payload.body.unwrap_or_default().value;
                let _: RouterResponse = serde_json::from_slice(&body_vec)?;
                self.config_addr.do_send(ConfigCmd::SetTmpValue(
                    source_req.config_key,
                    source_req.value,
                ));
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
                let payload = PayloadUtils::build_payload(RAFT_ROUTE_REQUEST, request);
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

///
/// raft 请求路由
/// 之前不同raft类型请求路由分别用不同的对象处理；
/// 后续考虑都使用这个对象统一处理；
#[derive(Clone)]
pub struct RaftRequestRoute {
    raft_addr_route: Arc<RaftAddrRouter>,
    cluster_sender: Arc<RaftClusterRequestSender>,
    raft: Arc<NacosRaft>,
    import_reader: Addr<TransferImportManager>,
}

impl RaftRequestRoute {
    pub fn new(
        raft_addr_route: Arc<RaftAddrRouter>,
        cluster_sender: Arc<RaftClusterRequestSender>,
        raft: Arc<NacosRaft>,
        import_reader: Addr<TransferImportManager>,
    ) -> Self {
        Self {
            raft_addr_route,
            cluster_sender,
            raft,
            import_reader,
        }
    }

    fn unknown_err(&self) -> anyhow::Error {
        anyhow::anyhow!("unknown the raft leader addr!")
    }

    pub async fn request_namespace(
        &self,
        req: NamespaceRaftReq,
    ) -> anyhow::Result<NamespaceRaftResult> {
        match self.raft_addr_route.get_route_addr().await? {
            RouteAddr::Local => {
                let resp = self
                    .raft
                    .client_write(ClientWriteRequest::new(ClientRequest::NamespaceReq(req)))
                    .await?;
                if let ClientResponse::Success = resp.data {
                    Ok(NamespaceRaftResult::None)
                } else {
                    Err(anyhow::anyhow!("response type is error!"))
                }
            }
            RouteAddr::Remote(_, addr) => {
                let req: RouterRequest = req.into();
                let request = serde_json::to_string(&req).unwrap_or_default();
                let payload = PayloadUtils::build_payload(RAFT_ROUTE_REQUEST, request);
                let resp_payload = self.cluster_sender.send_request(addr, payload).await?;
                let body_vec = resp_payload.body.unwrap_or_default().value;
                let resp: RouterResponse = serde_json::from_slice(&body_vec)?;
                match resp {
                    RouterResponse::NamespaceResult { result } => Ok(result),
                    _ => Err(anyhow::anyhow!("response type is error!")),
                }
            }
            RouteAddr::Unknown => Err(self.unknown_err()),
        }
    }

    pub async fn request_import(
        &self,
        data: Vec<u8>,
        param: TransferImportParam,
    ) -> anyhow::Result<TransferImportResponse> {
        match self.raft_addr_route.get_route_addr().await? {
            RouteAddr::Local => {
                self.import_reader
                    .send(TransferImportRequest::Import(data, param))
                    .await?
            }
            RouteAddr::Remote(_, addr) => {
                let req = RouterRequest::ImportData { data, param };
                let request = serde_json::to_string(&req).unwrap_or_default();
                let payload = PayloadUtils::build_payload(RAFT_ROUTE_REQUEST, request);
                let resp_payload = self.cluster_sender.send_request(addr, payload).await?;
                let body_vec = resp_payload.body.unwrap_or_default().value;
                let resp: RouterResponse = serde_json::from_slice(&body_vec)?;
                match resp {
                    RouterResponse::ImportResult { result } => Ok(result),
                    _ => Err(anyhow::anyhow!("response type is error!")),
                }
            }
            RouteAddr::Unknown => Err(self.unknown_err()),
        }
    }

    pub async fn request(&self, req: ClientRequest) -> anyhow::Result<ClientResponse> {
        match self.raft_addr_route.get_route_addr().await? {
            RouteAddr::Local => {
                let resp = self.raft.client_write(ClientWriteRequest::new(req)).await?;
                Ok(resp.data)
            }
            RouteAddr::Remote(_, addr) => {
                let req: RouterRequest = req.into();
                let router_resp = router_request(req, addr, &self.cluster_sender).await?;
                let resp: ClientResponse = router_resp.try_into()?;
                Ok(resp)
            }
            RouteAddr::Unknown => Err(self.unknown_err()),
        }
    }
}
