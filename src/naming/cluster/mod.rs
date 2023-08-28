
//distor cluster

use std::sync::Arc;

use crate::{common::appdata::AppShareData, naming::core::{NamingCmd, NamingResult}};

use self::model::{NamingRouteRequest, NamingRouterResponse};

pub mod node_manage;
pub mod route;
pub mod model;
pub mod sync_sender;



pub async fn handle_naming_route(
    app: &Arc<AppShareData>,
    req: NamingRouteRequest,
) -> anyhow::Result<NamingRouterResponse> {
    match req {
        NamingRouteRequest::Ping(node_id) => {
            //更新node_id节点活跃状态
            app.naming_node_manage.active_node(node_id);
        },
        NamingRouteRequest::UpdateInstance { instance,tag } => {
            let cmd = NamingCmd::Update(instance, tag);
            let _:NamingResult  = app.naming_addr.send(cmd).await??;
        },
        NamingRouteRequest::RemoveInstance { instance } => {
            let cmd = NamingCmd::Delete(instance);
            let _:NamingResult  = app.naming_addr.send(cmd).await??;
        },
        NamingRouteRequest::SyncUpdateInstance { instance } => {
            let cmd = NamingCmd::Update(instance, None);
            let _:NamingResult  = app.naming_addr.send(cmd).await??;
        },
        NamingRouteRequest::SyncRemoveInstance { instance } => {
            let cmd = NamingCmd::Delete(instance);
            let _:NamingResult  = app.naming_addr.send(cmd).await??;
        },
    };
    Ok(NamingRouterResponse::None)
}

