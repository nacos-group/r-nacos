use crate::common::appdata::AppShareData;
use crate::common::model::ApiResult;
use crate::console::model::cluster_model::ClusterNodeInfo;
use actix_web::{web, HttpResponse, Responder};
use std::sync::Arc;

pub async fn query_cluster_info(app: web::Data<Arc<AppShareData>>) -> impl Responder {
    let nodes = app.naming_node_manage.get_all_valid_nodes().await.unwrap();
    let leader_node = app.raft.current_leader().await;
    let mut list = vec![];
    for node in nodes {
        let mut node_info: ClusterNodeInfo = node.into();
        if let Some(leader_node) = &leader_node {
            if node_info.node_id == *leader_node {
                node_info.raft_leader = true;
            }
        }
        if app.sys_config.raft_node_id == node_info.node_id {
            node_info.current_node = true;
        }
        list.push(node_info);
    }
    HttpResponse::Ok().json(ApiResult::success(Some(list)))
}
