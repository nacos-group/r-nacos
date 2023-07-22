use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::Responder;
use async_raft::raft::{AppendEntriesRequest, InstallSnapshotRequest, VoteRequest};

use crate::common::appdata::AppData;
use crate::raft::asyncraft::store::ClientRequest;


// --- Raft communication

pub async fn vote(app: Data<AppData>, req: Json<VoteRequest>) -> actix_web::Result<impl Responder> {
    let res = app.raft.vote(req.0).await.unwrap();
    Ok(Json(res))
}

pub async fn append(app: Data<AppData>, req: Json<AppendEntriesRequest<ClientRequest>>) -> actix_web::Result<impl Responder> {
    let res = app.raft.append_entries(req.0).await.unwrap();
    Ok(Json(res))
}

pub async fn snapshot(
    app: Data<AppData>,
    req: Json<InstallSnapshotRequest>,
) -> actix_web::Result<impl Responder> {
    let res = app.raft.install_snapshot(req.0).await.unwrap();
    Ok(Json(res))
}
