use std::collections::BTreeMap;
use actix_web::{HttpResponse, Responder, web};
use actix_web::http::header;
use actix_web::web::Data;
use openraft::BasicNode;
use openraft::error::RaftError;
use openraft::raft::ClientWriteResponse;
use crate::common::appdata::AppData;
use crate::console::model::raft_model::{NodeInfo, NodeMember};
use crate::raft::store::{NodeId, TypeConfig};

pub async fn raft_add_learner(
    app: Data<AppData>,
    req: web::Form<NodeInfo>,
) -> impl Responder {
    let node_info = req.0;
    let node = BasicNode { addr: node_info.node_addr.unwrap() };
    let res = app.raft.add_learner(node_info.node_id.unwrap(), node, true).await;
    match res {
        Ok(res) => {
            let v = serde_json::to_string(&res).unwrap();
            HttpResponse::Ok()
                .insert_header(header::ContentType(mime::APPLICATION_JSON))
                .body(v)
        }
        Err(err) => {
            HttpResponse::InternalServerError().body(err.to_string())
        }
    }
}

pub async fn raft_init(app: Data<AppData>) -> impl Responder {
    let mut nodes = BTreeMap::new();
    nodes.insert(app.sys_config.raft_node_id.to_owned(), BasicNode { addr: app.sys_config.raft_node_addr.clone() });
    let res = app.raft.initialize(nodes).await;
    match res {
        Ok(res) => {
            let v = serde_json::to_string(&res).unwrap();
            HttpResponse::Ok()
                .insert_header(header::ContentType(mime::APPLICATION_JSON))
                .body(v)
        }
        Err(err) => {
            HttpResponse::InternalServerError().body(err.to_string())
        }
    }
}

pub async fn raft_change_membership(
    app: Data<AppData>,
    req: web::Form<NodeMember>
) -> impl Responder {
    let node_member = req.0;
    let res = app.raft.change_membership(node_member.node_ids, false).await;
    match res {
        Ok(res) => {
            let v = serde_json::to_string(&res).unwrap();
            HttpResponse::Ok()
                .insert_header(header::ContentType(mime::APPLICATION_JSON))
                .body(v)
        }
        Err(err) => {
            HttpResponse::InternalServerError().body(err.to_string())
        }
    }
}

pub async fn raft_metrics(app: Data<AppData>) -> impl Responder {
    let metrics = app.raft.metrics().borrow().clone();
    let v = serde_json::to_string(&metrics).unwrap();
    HttpResponse::Ok()
        .insert_header(header::ContentType(mime::APPLICATION_JSON))
        .body(v)
}