use crate::common::appdata::AppShareData;
use crate::mcp::model::actor_model::{McpManagerReq, McpManagerResult};
use crate::mcp::model::sse_model::{SseConnMetaInfo, SseStreamManageAsyncCmd, SseStreamManageCmd};
use crate::mcp::sse_manage::SseConnUtils;
use crate::openapi::mcp::model::{McpPath, SseMessagePath};
use actix_web::http::StatusCode;
use actix_web::{web, HttpRequest, HttpResponse};
use bytes::Bytes;
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

pub async fn sse_connect(
    req: HttpRequest,
    path: web::Path<McpPath>,
    app_share_data: web::Data<Arc<AppShareData>>,
) -> actix_web::Result<HttpResponse> {
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
    let (tx, rx) = tokio::sync::mpsc::channel::<anyhow::Result<Bytes>>(3);
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
        "event: endpoint\ndata: /rnacos/mcp/sse/messages/{}/{}\n\n",
        path.server_key, &session_id
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
    body: web::Json<Value>,
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
    let message = match super::api::handle_request(
        &app_share_data,
        body,
        &mcp_server,
        &path.session_id,
    )
    .await
    {
        Ok(result) => SseConnUtils::create_sse_message(&result),
        Err(err) => {
            return err;
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
