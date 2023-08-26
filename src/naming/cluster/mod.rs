
//distor cluster

use std::sync::Arc;

use crate::{common::appdata::AppShareData, naming::core::{NamingCmd, NamingResult}};

use self::model::{NamingRouterRequest, NamingRouterResponse};

pub mod node_manage;
pub mod route;
pub mod model;



pub async fn handle_naming_route(
    app: &Arc<AppShareData>,
    req: NamingRouterRequest,
) -> anyhow::Result<NamingRouterResponse> {
    match req {
        NamingRouterRequest::Ping(node_id) => {
            //更新node_id节点活跃状态
            todo!()
        },
        NamingRouterRequest::UpdateInstance { instance,tag } => {
            let cmd = NamingCmd::Update(instance, tag);
            let _:NamingResult  = app.naming_addr.send(cmd).await??;
        },
        NamingRouterRequest::RemoveInstance { instance } => {
            let cmd = NamingCmd::Delete(instance);
            let _:NamingResult  = app.naming_addr.send(cmd).await??;
        },
        NamingRouterRequest::SyncUpdateInstance { instance } => {
            let cmd = NamingCmd::Update(instance, None);
            let _:NamingResult  = app.naming_addr.send(cmd).await??;
        },
        NamingRouterRequest::SyncRemoveInstance { instance } => {
            let cmd = NamingCmd::Delete(instance);
            let _:NamingResult  = app.naming_addr.send(cmd).await??;
        },
    };
    Ok(NamingRouterResponse::None)
}

