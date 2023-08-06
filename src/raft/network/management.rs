use std::collections::HashSet;
use std::sync::Arc;

use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::Responder;
use async_raft::raft::ClientWriteRequest;

use crate::common::appdata::AppShareData;
use crate::raft::store::ClientRequest;
use crate::raft::store::NodeId;
use crate::raft::join_node;


// --- Cluster management

pub async fn join_learner(app: Data<Arc<AppShareData>>, req: Json<(NodeId, String)>) -> actix_web::Result<impl Responder> {
    let node_id = req.0 .0;
    let addr = Arc::new(req.0 .1);
    app.raft.client_write(ClientWriteRequest::new(ClientRequest::NodeAddr { id: node_id, addr})).await.unwrap();
    app.raft.add_non_voter(node_id).await.unwrap();
    join_node(app.raft.as_ref(),app.raft_store.as_ref(),node_id).await.ok();
    Ok("{\"ok\":1}")
}

/// Add a node as **Learner**.
///
/// A Learner receives log replication from the leader but does not vote.
/// This should be done before adding a node as a member into the cluster
/// (by calling `change-membership`)
//#[post("/add-learner")]
pub async fn add_learner(app: Data<Arc<AppShareData>>, req: Json<(NodeId, String)>) -> actix_web::Result<impl Responder> {
    let node_id = req.0 .0;
    let addr = Arc::new(req.0 .1);
    app.raft.client_write(ClientWriteRequest::new(ClientRequest::NodeAddr { id: node_id, addr})).await.unwrap();
    app.raft.add_non_voter(node_id).await.unwrap();
    Ok("{\"ok\":1}")
}

/// Changes specified learners to members, or remove members.
//#[post("/change-membership")]
pub async fn change_membership(app: Data<Arc<AppShareData>>, req: Json<HashSet<NodeId>>) -> actix_web::Result<impl Responder> {
    app.raft.change_membership(req.0).await.unwrap();
    Ok("{\"ok\":1}")
}

/// Initialize a single-node cluster.
//#[post("/init")]
pub async fn init(app: Data<Arc<AppShareData>>) -> actix_web::Result<impl Responder> {
    let mut members = HashSet::new();
    let node_id = app.sys_config.raft_node_id.to_owned();
    members.insert(node_id);
    app.raft.initialize(members).await.ok();
    app.raft.client_write(ClientWriteRequest::new(ClientRequest::NodeAddr { id:node_id, addr: Arc::new(app.sys_config.raft_node_addr.to_owned())})).await.unwrap();
    Ok("{\"ok\":1}")
}

/// Get the latest metrics of the cluster
//#[get("/metrics")]
pub async fn metrics(app: Data<Arc<AppShareData>>) -> actix_web::Result<impl Responder> {
    let metrics = app.raft.metrics().borrow().clone();
    Ok(Json(metrics))
}
