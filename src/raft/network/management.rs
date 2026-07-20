use std::collections::HashSet;
use std::sync::Arc;

use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::{HttpResponse, Responder};
use async_raft_ext::raft::ClientWriteRequest;
use serde_json::json;

use crate::common::appdata::AppShareData;
use crate::raft::join_node;
use crate::raft::store::ClientRequest;
use crate::raft::store::NodeId;

// --- Cluster management

pub async fn join_learner(
    app: Data<Arc<AppShareData>>,
    req: Json<(NodeId, String)>,
) -> actix_web::Result<impl Responder> {
    let node_id = req.0 .0;
    let addr = Arc::new(req.0 .1);
    app.raft
        .client_write(ClientWriteRequest::new(ClientRequest::NodeAddr {
            id: node_id,
            addr,
        }))
        .await
        .unwrap();
    app.raft.add_non_voter(node_id).await.unwrap();
    join_node(app.raft.as_ref(), app.raft_store.as_ref(), node_id)
        .await
        .ok();
    Ok("{\"ok\":1}")
}

/// Add a node as **Learner**.
///
/// A Learner receives log replication from the leader but does not vote.
/// This should be done before adding a node as a member into the cluster
/// (by calling `change-membership`)
//#[post("/add-learner")]
pub async fn add_learner(
    app: Data<Arc<AppShareData>>,
    req: Json<(NodeId, String)>,
) -> actix_web::Result<impl Responder> {
    let node_id = req.0 .0;
    let addr = Arc::new(req.0 .1);
    app.raft
        .client_write(ClientWriteRequest::new(ClientRequest::NodeAddr {
            id: node_id,
            addr,
        }))
        .await
        .unwrap();
    app.raft.add_non_voter(node_id).await.unwrap();
    Ok("{\"ok\":1}")
}

/// Changes specified learners to members, or remove members.
//#[post("/change-membership")]
pub async fn change_membership(
    app: Data<Arc<AppShareData>>,
    req: Json<HashSet<NodeId>>,
) -> actix_web::Result<impl Responder> {
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
    app.raft
        .client_write(ClientWriteRequest::new(ClientRequest::NodeAddr {
            id: node_id,
            addr: Arc::new(app.sys_config.raft_node_addr.to_owned()),
        }))
        .await
        .unwrap();
    Ok("{\"ok\":1}")
}

/// Get the latest metrics of the cluster
//#[get("/metrics")]
pub async fn metrics(app: Data<Arc<AppShareData>>) -> actix_web::Result<impl Responder> {
    let metrics = app.raft.metrics().borrow().clone();
    Ok(Json(metrics))
}

/// 触发 raft 禁写：校验 `{local_db_dir}/close_raft_mark` 标记文件存在后，置禁写标记
//#[post("/close-write")]
pub async fn close_write(app: Data<Arc<AppShareData>>) -> actix_web::Result<impl Responder> {
    let local_db_dir = &app.sys_config.local_db_dir;
    if !close_raft_mark_exists(local_db_dir.as_str()) {
        log::warn!(
            "close_write rejected, mark file not found: {}/close_raft_mark",
            local_db_dir
        );
        return Ok(HttpResponse::Ok().json(json!({
            "ok": 0,
            "msg": format!("执行前需要在指定目录({})创建对应标记文件 close_raft_mark", local_db_dir)
        })));
    }
    app.raft_store.set_close_write();
    log::info!(
        "close_write enabled, raft write disabled, local_db_dir:{}",
        local_db_dir
    );
    Ok(HttpResponse::Ok().json(json!({ "ok": 1 })))
}

/// raft 日志写入错误注入请求体（仅 debug feature）
#[cfg(feature = "debug")]
#[derive(serde::Deserialize)]
pub struct InjectErrorReq {
    /// 注入场景，目前仅支持 discard_log
    pub scene: String,
    /// 丢弃后续 WriteBatch 的次数
    pub times: u64,
}

/// 触发 raft 日志写入错误注入（仅 debug feature，免鉴权）
//#[post("/inject-error")]
#[cfg(feature = "debug")]
pub async fn inject_error(
    app: Data<Arc<AppShareData>>,
    req: Json<InjectErrorReq>,
) -> actix_web::Result<impl Responder> {
    match req.scene.as_str() {
        "discard_log" => match app.raft_store.inject_discard_log(req.times).await {
            Ok(_) => {
                log::warn!(
                    "inject_error discard_log ok, node:{} times:{}",
                    app.sys_config.raft_node_id,
                    req.times
                );
                Ok(HttpResponse::Ok().json(json!({ "ok": 1 })))
            }
            Err(e) => {
                log::error!(
                    "inject_error discard_log failed, node:{} times:{} err:{:?}",
                    app.sys_config.raft_node_id,
                    req.times,
                    e
                );
                Ok(HttpResponse::Ok().json(json!({ "ok": 0, "msg": format!("{:?}", e) })))
            }
        },
        other => {
            log::warn!("inject_error unknown scene:{}", other);
            Ok(HttpResponse::Ok()
                .json(json!({ "ok": 0, "msg": format!("unknown scene: {other}") })))
        }
    }
}

/// 校验禁写安全闸门标记文件 `{local_db_dir}/close_raft_mark` 是否存在
fn close_raft_mark_exists(local_db_dir: &str) -> bool {
    std::path::Path::new(local_db_dir)
        .join("close_raft_mark")
        .exists()
}

#[cfg(test)]
mod tests {
    use super::close_raft_mark_exists;
    use tempfile::tempdir;

    #[test]
    fn close_raft_mark_not_exists_when_absent() {
        let dir = tempdir().unwrap();
        assert!(!close_raft_mark_exists(dir.path().to_str().unwrap()));
    }

    #[test]
    fn close_raft_mark_exists_when_present() {
        let dir = tempdir().unwrap();
        let mark = dir.path().join("close_raft_mark");
        std::fs::write(&mark, b"").unwrap();
        assert!(close_raft_mark_exists(dir.path().to_str().unwrap()));
    }
}
