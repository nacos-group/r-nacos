use std::sync::Arc;

use async_raft::raft::ClientWriteRequest;

use crate::{
    common::appdata::AppShareData,
    config::core::{ConfigAsyncCmd, ConfigKey},
};

use self::model::{RouterRequest, RouterResponse};

use super::{join_node, store::ClientRequest};

pub mod model;
pub mod route;
pub mod routeapi;

pub async fn handle_route(
    app: &Arc<AppShareData>,
    req: RouterRequest,
) -> anyhow::Result<RouterResponse> {
    match req {
        RouterRequest::ConfigSet {
            key,
            value,
            extend_info: _,
        } => {
            let config_key: ConfigKey = (&key as &str).into();
            app.config_addr
                .send(ConfigAsyncCmd::Add(config_key, value))
                .await??;
        }
        RouterRequest::ConfigDel {
            key,
            extend_info: _,
        } => {
            let config_key: ConfigKey = (&key as &str).into();
            app.config_addr
                .send(ConfigAsyncCmd::Delete(config_key))
                .await??;
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
        }
    };
    Ok(RouterResponse::None)
}
