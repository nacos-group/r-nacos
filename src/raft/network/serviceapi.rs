
use actix_web::post;
use actix_web::web;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::Responder;
use openraft::raft::AppendEntriesRequest;
use openraft::raft::InstallSnapshotRequest;
use openraft::raft::VoteRequest;

use crate::common::appdata::AppData;
use crate::raft::store::NodeId;
use crate::raft::store::TypeConfig;

// --- Raft communication

//#[post("/raft-vote")]
pub async fn vote(app: Data<AppData>, req: Json<VoteRequest<NodeId>>) -> actix_web::Result<impl Responder> {
    let res = app.raft.vote(req.0).await;
    Ok(Json(res))
}

//#[post("/raft-append")]
pub async fn append(app: Data<AppData>, req: Json<AppendEntriesRequest<TypeConfig>>) -> actix_web::Result<impl Responder> {
    let res = app.raft.append_entries(req.0).await;
    Ok(Json(res))
}

//#[post("/raft-snapshot")]
pub async fn snapshot(
    app: Data<AppData>,
    req: Json<InstallSnapshotRequest<TypeConfig>>,
) -> actix_web::Result<impl Responder> {
    let res = app.raft.install_snapshot(req.0).await;
    Ok(Json(res))
}


pub fn raft_config(config: &mut web::ServiceConfig) {
    config.service(
    web::scope("/nacos/v1/raft")
        .service(web::resource("/raft-vote").route(web::post().to(vote)))
        .service(web::resource("/raft-append").route(web::post().to(append)))
        .service(web::resource("/raft-snapshot").route(web::post().to(snapshot)))
    );
}