
//distor cluster

use std::{sync::Arc, collections::HashMap, process::id};

use crate::{common::appdata::AppShareData, naming::core::{NamingCmd, NamingResult}};

use self::{model::{NamingRouteRequest, NamingRouterResponse}, node_manage::NodeManageRequest};

pub mod node_manage;
pub mod route;
pub mod model;
pub mod sync_sender;



pub async fn handle_naming_route(
    app: &Arc<AppShareData>,
    req: NamingRouteRequest,
    extend_info: HashMap<String, String>,
) -> anyhow::Result<NamingRouterResponse> {
    let cluster_id:u64 = if let Some(id_str) = extend_info.get("cluster_id") {
        match id_str.parse() {
            Ok(id) => id,
            Err(err) => {
                return Err(anyhow::anyhow!("cluster_id can't parse to u64,{}",id_str))
            },
        }
    }
    else{
        return Err(anyhow::anyhow!("extend_info not found node_id"));
    };
    match req {
        NamingRouteRequest::Ping(cluster_id) => {
            //更新node_id节点活跃状态
            app.naming_node_manage.active_node(cluster_id);
        },
        NamingRouteRequest::UpdateInstance { instance,tag } => {
            let cmd = NamingCmd::Update(instance, tag);
            let _:NamingResult  = app.naming_addr.send(cmd).await??;
        },
        NamingRouteRequest::RemoveInstance { instance } => {
            let cmd = NamingCmd::Delete(instance);
            let _:NamingResult  = app.naming_addr.send(cmd).await??;
        },
        NamingRouteRequest::SyncUpdateInstance {mut instance } => {
            app.naming_inner_node_manage.do_send(NodeManageRequest::AddClientId(cluster_id, instance.client_id.clone()));
            instance.from_cluster = cluster_id;
            let cmd = NamingCmd::Update(instance, None);
            let _:NamingResult  = app.naming_addr.send(cmd).await??;
        },
        NamingRouteRequest::SyncRemoveInstance { mut instance } => {
            app.naming_node_manage.active_node(cluster_id);
            instance.from_cluster = cluster_id;
            let cmd = NamingCmd::Delete(instance);
            let _:NamingResult  = app.naming_addr.send(cmd).await??;
        },
    };
    Ok(NamingRouterResponse::None)
}

