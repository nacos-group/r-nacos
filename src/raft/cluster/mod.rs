use std::sync::Arc;

use async_raft_ext::raft::ClientWriteRequest;

use self::model::{RouterRequest, RouterResponse};
use super::{db::table::TableManagerAsyncReq, join_node, store::ClientRequest};
use crate::grpc::handler::RAFT_ROUTE_REQUEST;
use crate::grpc::PayloadUtils;
use crate::namespace::model::NamespaceRaftResult;
use crate::raft::network::factory::RaftClusterRequestSender;
use crate::raft::store::ClientResponse;
use crate::transfer::model::TransferImportRequest;
use crate::{
    common::appdata::AppShareData,
    config::core::{ConfigAsyncCmd, ConfigKey},
};

pub mod model;
pub mod route;
pub mod routeapi;

pub async fn handle_route(
    app: &Arc<AppShareData>,
    req: RouterRequest,
) -> anyhow::Result<RouterResponse> {
    match req {
        RouterRequest::RaftRequest(req) => {
            let r = app.raft.client_write(ClientWriteRequest::new(req)).await?;
            Ok(RouterResponse::RaftResponse(r.data))
        }
        RouterRequest::ConfigSet {
            key,
            value,
            op_user,
            config_type,
            desc,
            extend_info: _,
        } => {
            let config_key: ConfigKey = (&key as &str).into();
            app.config_addr
                .send(ConfigAsyncCmd::Add {
                    key: config_key,
                    value,
                    op_user,
                    config_type,
                    desc,
                })
                .await??;
            Ok(RouterResponse::None)
        }
        RouterRequest::ConfigDel {
            key,
            extend_info: _,
        } => {
            let config_key: ConfigKey = (&key as &str).into();
            app.config_addr
                .send(ConfigAsyncCmd::Delete(config_key))
                .await??;
            Ok(RouterResponse::None)
        }
        RouterRequest::JoinNode {
            node_id,
            node_addr: addr,
        } => {
            app.raft
                .client_write(ClientWriteRequest::new(ClientRequest::NodeAddr {
                    id: node_id,
                    addr,
                }))
                .await?;
            app.raft.add_non_voter(node_id).await?;
            join_node(app.raft.as_ref(), app.raft_store.as_ref(), node_id).await?;
            Ok(RouterResponse::None)
        }
        RouterRequest::TableManagerReq { req } => {
            let result = app
                .raft_table_manage
                .send(TableManagerAsyncReq(req))
                .await??;
            Ok(RouterResponse::TableManagerResult { result })
        }
        RouterRequest::TableManagerQueryReq { req } => {
            let result = app.raft_table_manage.send(req).await??;
            Ok(RouterResponse::TableManagerResult { result })
        }
        RouterRequest::CacheLimiterReq { req } => {
            let result = app.cache_manager.send(req).await??;
            Ok(RouterResponse::CacheManagerResult { result })
        }
        RouterRequest::NamespaceReq { req } => {
            let resp = app
                .raft
                .client_write(ClientWriteRequest::new(ClientRequest::NamespaceReq(req)))
                .await?;
            if let ClientResponse::Success = resp.data {
                return Ok(RouterResponse::NamespaceResult {
                    result: NamespaceRaftResult::None,
                });
            }
            Ok(RouterResponse::None)
        }
        RouterRequest::ImportData { data, param } => {
            let result = app
                .transfer_import_manager
                .send(TransferImportRequest::Import(data, param))
                .await??;
            Ok(RouterResponse::ImportResult { result })
        }
    }
}

pub async fn router_request(
    req: RouterRequest,
    addr: Arc<String>,
    cluster_sender: &Arc<RaftClusterRequestSender>,
) -> anyhow::Result<RouterResponse> {
    let request = serde_json::to_string(&req).unwrap_or_default();
    let payload = PayloadUtils::build_payload(RAFT_ROUTE_REQUEST, request);
    let resp_payload = cluster_sender.send_request(addr, payload).await?;
    let body_vec = resp_payload.body.unwrap_or_default().value;
    let router_resp: RouterResponse = serde_json::from_slice(&body_vec)?;
    Ok(router_resp)
}
