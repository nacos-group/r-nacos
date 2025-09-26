use crate::common::appdata::AppShareData;
use crate::grpc::handler::NAMING_ROUTE_REQUEST;
use crate::grpc::PayloadUtils;
use crate::mcp::model::actor_model::{McpManagerReq, McpManagerResult};
use crate::mcp::model::sse_model::{SseConnMetaInfo, SseStreamManageAsyncCmd, SseStreamManageCmd};
use crate::mcp::sse_manage::SseConnUtils;
use crate::naming::cluster::model::{NamingRouteRequest, NamingRouterResponse};
use crate::openapi::mcp::model::{JsonRpcRequest, McpPath, SseMessagePath};
use crate::openapi::mcp::{HandleOtherResult, IGNORE_TRASFER_HEADERS};
use actix_web::http::StatusCode;
use actix_web::{web, HttpRequest, HttpResponse};
use bytes::Bytes;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

pub async fn sse_connect(
    _req: HttpRequest,
    path: web::Path<McpPath>,
    app_share_data: web::Data<Arc<AppShareData>>,
) -> actix_web::Result<HttpResponse> {
    /*
    // 部分client 没有设置accept，这里为有兼容性暂时不校验
    if let Some(accept_header) = req.headers().get("accept") {
        let accept_value = accept_header.to_str().unwrap_or("");

        // 检查是否包含 text/event-stream
        if !accept_value.contains("text/event-stream") {
            return Ok(HttpResponse::BadRequest()
                .body(r#"error: Unsupported Accept header. Must contain 'text/event-stream'"#));
        }
    } else {
        // 如果没有 Accept 请求头，也返回错误
        return Ok(HttpResponse::BadRequest()
            .body(r#"error: Unsupported Accept header. Must contain 'text/event-stream'"#));
    }
    */
    //校验path信息是否合法
    let mcp_server = if let Ok(Ok(McpManagerResult::ServerInfo(Some(server)))) = app_share_data
        .mcp_manager
        .send(McpManagerReq::GetServerByKey(path.server_key.clone()))
        .await
    {
        server
    } else {
        return Ok(HttpResponse::BadRequest().body(r#"error: McpServer not found"#));
    };
    if !mcp_server.auth_keys.contains(&path.auth_key) {
        return Ok(HttpResponse::BadRequest().body(r#"error: Invalid auth key"#));
    }
    let (tx, rx) = tokio::sync::mpsc::channel::<anyhow::Result<Bytes>>(10);
    let session_id = Arc::new(Uuid::new_v4().to_string().replace("-", ""));
    let meta = SseConnMetaInfo {
        session_id: session_id.clone(),
        mcp_server_key: path.server_key.clone(),
    };
    app_share_data
        .sse_stream_manager
        .send(SseStreamManageCmd::AddConn(meta, tx))
        .await
        .ok();
    let init_message = format!(
        "event: endpoint\ndata: /rnacos/mcp/sse/messages/{}/{}/{}\n\n",
        app_share_data.sys_config.raft_node_id, path.server_key, &session_id
    );
    app_share_data
        .sse_stream_manager
        .send(SseStreamManageAsyncCmd::SendMessage(
            session_id,
            init_message,
        ))
        .await
        .ok();

    let r_stream = tokio_stream::wrappers::ReceiverStream::new(rx);
    let mut resp_builder = HttpResponse::build(StatusCode::OK);
    resp_builder
        //.insert_header(("connection", "keep-alive"))
        .insert_header(("content-type", "text/event-stream; charset=utf-8"))
        .insert_header(("cache-control", "no-cache"))
        .insert_header(("x-accel-buffering", "no"))
        .insert_header(("transfer-encoding", "chunked"))
        .insert_header(("server", "r-nacos"));
    Ok(resp_builder.streaming(r_stream))
}

pub async fn sse_message(
    req: HttpRequest,
    body: web::Json<JsonRpcRequest>,
    path: web::Path<SseMessagePath>,
    app_share_data: web::Data<Arc<AppShareData>>,
) -> actix_web::Result<HttpResponse> {
    let mcp_server = if let Ok(Ok(McpManagerResult::ServerInfo(Some(server)))) = app_share_data
        .mcp_manager
        .send(McpManagerReq::GetServerByKey(path.server_key.clone()))
        .await
    {
        server
    } else {
        return Ok(HttpResponse::NotFound().body(r#"error: McpServer not found"#));
    };
    if path.node_id != app_share_data.sys_config.raft_node_id {
        let mut headers = HashMap::new();
        for (key, value) in req.headers() {
            if IGNORE_TRASFER_HEADERS.contains(&key.as_str()) {
                continue;
            }
            headers.insert(
                key.to_string(),
                String::from_utf8_lossy(value.as_bytes()).to_string(),
            );
        }
        return match post_to_remote(&app_share_data, &path, body.into_inner(), headers).await {
            Ok(_) => Ok(HttpResponse::Accepted().body("Accepted")),
            Err(e) => Ok(HttpResponse::InternalServerError().body(format!("error: {}", e))),
        };
    }
    let mut headers = HashMap::new();
    for (key, value) in req.headers() {
        if IGNORE_TRASFER_HEADERS.contains(&key.as_str()) {
            continue;
        }
        headers.insert(key.as_str(), value.as_bytes());
    }
    let message = match super::api::handle_request(
        &app_share_data,
        body.into_inner(),
        &mcp_server,
        &path.session_id,
        headers,
    )
    .await
    {
        Ok(result) => SseConnUtils::create_sse_message(&result),
        Err(e) => {
            match e {
                HandleOtherResult::Accepted => {
                    return Ok(HttpResponse::Accepted().body(""));
                }
            };
        }
    };
    app_share_data
        .sse_stream_manager
        .send(SseStreamManageAsyncCmd::SendMessage(
            path.session_id.clone(),
            message,
        ))
        .await
        .ok();
    Ok(HttpResponse::Accepted().body("Accepted"))
}

async fn post_to_remote(
    app_share_data: &Arc<AppShareData>,
    path: &SseMessagePath,
    request: JsonRpcRequest,
    headers: HashMap<String, String>,
) -> anyhow::Result<()> {
    let addr = app_share_data
        .naming_node_manage
        .get_node_addr(path.node_id)
        .await?;

    let req = NamingRouteRequest::McpMessages {
        server_key: path.server_key.clone(),
        session_id: path.session_id.clone(),
        request,
        headers,
    };
    let request = serde_json::to_string(&req).unwrap_or_default();
    let payload = PayloadUtils::build_payload(NAMING_ROUTE_REQUEST, request);
    let resp_payload = app_share_data
        .cluster_sender
        .send_request(addr.clone(), payload)
        .await?;
    let body_vec = resp_payload.body.unwrap_or_default().value;
    let _: NamingRouterResponse = serde_json::from_slice(&body_vec)?;
    Ok(())
}
