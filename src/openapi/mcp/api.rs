use super::model::{JsonRpcError, JsonRpcRequest, JsonRpcResponse, McpPath};
use crate::common::appdata::AppShareData;
use crate::common::get_app_version;
use crate::mcp::model::actor_model::{McpManagerReq, McpManagerResult};
use crate::mcp::model::mcp::McpServer;
use crate::mcp::model::tools::{ConvertType, McpTool, ToolFunctionValue};
use crate::naming::core::{NamingCmd, NamingResult};
use crate::naming::model::ServiceKey;
use crate::openapi::mcp::{HandleOtherResult, IGNORE_TRASFER_HEADERS};
use actix_web::{web, HttpRequest, HttpResponse, Result};
use serde_json::{json, Value};

use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Arc;
use std::time::SystemTime;
use uuid::Uuid;

/// MCP 请求日志参数
#[derive(Debug, Clone)]
pub enum McpHandleLogArgs {
    /// 日志参数内容为空
    None,
    /// 忽略,不打印日志
    Ignore,
    /// 打印日志
    Arg(String),
}

impl Display for McpHandleLogArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            McpHandleLogArgs::None => "".to_string(),
            McpHandleLogArgs::Ignore => "".to_string(),
            McpHandleLogArgs::Arg(arg) => arg.to_string(),
        };
        write!(f, "{}", str)
    }
}

impl McpHandleLogArgs {
    pub fn enable_log(&self) -> bool {
        match self {
            McpHandleLogArgs::None => true,
            McpHandleLogArgs::Ignore => false,
            McpHandleLogArgs::Arg(_) => true,
        }
    }
}
// 统一的流式返回处理函数
/*
fn create_streaming_response(response: JsonRpcResponse, session_id: String) -> Result<HttpResponse> {
    let json_string = serde_json::to_string(&response).unwrap_or_default();
    let sse_content = format!("event: message\ndata: {}\n\n", json_string);

    // 创建流式响应
    let r_stream = stream::once(async move { Ok::<Bytes, actix_web::Error>(Bytes::from(sse_content)) });
    let mut client_resp = HttpResponse::Ok();
    client_resp
        .content_type("text/event-stream")
        .insert_header((actix_web::http::header::HeaderName::from_bytes(b"mcp-session-id").unwrap(), actix_web::http::header::HeaderValue::from_bytes(session_id.as_bytes()).unwrap()))
        .insert_header(("server","actix-web"))
        .insert_header(("cache-control","no-cache, no-transform"))
        .insert_header(("connection","keep-alive"))
        .insert_header(("x-accel-buffering","no"))
    Ok(client_resp.streaming(r_stream))
}
*/

pub async fn mcp_handler(
    req: HttpRequest,
    path: web::Path<McpPath>,
    app_share_data: web::Data<Arc<AppShareData>>,
    body: web::Json<Value>,
) -> Result<HttpResponse> {
    // 检查 Accept 请求头
    if let Some(accept_header) = req.headers().get("accept") {
        let accept_value = accept_header.to_str().unwrap_or("");

        // 检查是否包含 application/json 或 text/event-stream
        if !accept_value.contains("application/json") && !accept_value.contains("text/event-stream")
        {
            return Ok(HttpResponse::BadRequest()
                .content_type("application/json")
                .body(r#"{"error": "Unsupported Accept header. Must contain 'application/json' or 'text/event-stream'"}"#));
        }
    } else {
        // 如果没有 Accept 请求头，也返回错误
        return Ok(HttpResponse::BadRequest()
            .content_type("application/json")
            .body(r#"{"error": "Missing Accept header. Must contain 'application/json' or 'text/event-stream'"}"#));
    }
    //校验path信息是否合法
    let mcp_server = if let Ok(Ok(McpManagerResult::ServerInfo(Some(server)))) = app_share_data
        .mcp_manager
        .send(McpManagerReq::GetServerByKey(path.server_key.clone()))
        .await
    {
        server
    } else {
        return Ok(HttpResponse::BadRequest()
            .content_type("application/json")
            .body(r#"{"error": "McpServer not found"}"#));
    };
    if !mcp_server.auth_keys.contains(&path.auth_key) {
        return Ok(HttpResponse::BadRequest()
            .content_type("application/json")
            .body(r#"{"error": "Invalid auth key"}"#));
    }

    // 获取或生成 mcp-session-id
    let old_session_id = req
        .headers()
        .get("mcp-session-id")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());
    //let request_session_is_empty = old_session_id.is_none();
    let session_id = if old_session_id.is_some() {
        old_session_id.unwrap()
    } else {
        Uuid::new_v4().to_string().replace("-", "")
    };

    // 解析 JSON-RPC 请求
    let request: JsonRpcRequest = match serde_json::from_value(body.into_inner()) {
        Ok(req) => req,
        Err(_) => {
            return Ok(HttpResponse::Ok()
                .content_type("application/json")
                .insert_header(("mcp-session-id", session_id))
                .json(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: "Parse error".to_string(),
                        data: None,
                    }),
                    id: None,
                }));
        }
    };

    let mut headers = HashMap::new();
    for (key, value) in req.headers() {
        if IGNORE_TRASFER_HEADERS.contains(&key.as_str()) {
            continue;
        }
        headers.insert(key.as_str(), value.as_bytes());
    }
    let rpc_response =
        match handle_request(&app_share_data, request, &mcp_server, &session_id, headers).await {
            Ok(value) => value,
            Err(e) => {
                match e {
                    HandleOtherResult::Accepted => {
                        return Ok(HttpResponse::Accepted()
                            //.content_type("application/json")
                            .insert_header(("mcp-session-id", session_id))
                            .body(""));
                    }
                };
            }
        };
    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .insert_header(("mcp-session-id", session_id))
        .json(rpc_response))
}

pub async fn handle_request(
    app_share_data: &Arc<AppShareData>,
    request: JsonRpcRequest,
    mcp_server: &Arc<McpServer>,
    session_id: &String,
    headers: HashMap<&str, &[u8]>,
) -> Result<JsonRpcResponse, HandleOtherResult> {
    let start = SystemTime::now();
    let request_log_info = format!("|mcp|client_request|{}|{}", session_id, &request.method);

    // 验证 JSON-RPC 版本
    if request.jsonrpc != "2.0" {
        let duration = SystemTime::now()
            .duration_since(start)
            .unwrap_or_default()
            .as_secs_f64();
        log::error!(
            "{}|err|{}|invalid_jsonrpc_version",
            request_log_info,
            duration
        );
        return Ok(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32600,
                message: "Invalid Request".to_string(),
                data: None,
            }),
            id: request.id,
        });
    }

    // 收集日志参数
    let mut log_args = McpHandleLogArgs::None;

    // 根据不同的 method 处理请求
    let rpc_response = match request.method.as_str() {
        "notifications/initialized" => {
            // notifications/initialized 使用 JSON 格式（非流式）
            let duration = SystemTime::now()
                .duration_since(start)
                .unwrap_or_default()
                .as_secs_f64();
            log::info!(
                "{}|ok|{}|notifications_initialized",
                request_log_info,
                duration
            );
            return Err(HandleOtherResult::Accepted);
        }
        "initialize" => {
            // initialize 使用 SSE 格式的流式返回
            log_args = McpHandleLogArgs::Arg("initialize".to_string());
            handle_initialize(request.params, request.id)
        }
        "tools/call" => {
            // tools/call 使用 SSE 格式的流式返回
            match handle_tools_call(
                request.params,
                request.id.clone(),
                &mcp_server,
                &app_share_data,
                headers,
                &mut log_args,
            )
            .await
            {
                Ok(response) => response,
                Err(error) => {
                    let duration = SystemTime::now()
                        .duration_since(start)
                        .unwrap_or_default()
                        .as_secs_f64();
                    log::error!(
                        "{}|err|{}|{}|tools_call_error",
                        request_log_info,
                        duration,
                        &log_args
                    );
                    JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32000,
                            message: error.to_string(),
                            data: None,
                        }),
                        id: request.id,
                    }
                }
            }
        }
        "tools/list" => {
            // tools/list
            log_args = McpHandleLogArgs::Arg(format!(
                "tools_list:count:{}",
                mcp_server.release_value.tools.len()
            ));
            handle_tools_list(request.id, &mcp_server)
        }
        "resources/list" => {
            // resources/list
            log_args = McpHandleLogArgs::Arg("resources_list:empty".to_string());
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(json!({ "resources": [] })),
                error: None,
                id: request.id,
            }
        }
        "resources/templates/list" => {
            // resources/templates/list
            log_args = McpHandleLogArgs::Arg("resources_templates_list:empty".to_string());
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(json!({ "resourceTemplates": [] })),
                error: None,
                id: request.id,
            }
        }
        "prompts/list" => {
            log_args = McpHandleLogArgs::Arg("prompts_list:empty".to_string());
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(json!({ "prompts": [] })),
                error: None,
                id: request.id,
            }
        }
        "ping" => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(json!({})),
            error: None,
            id: request.id,
        },
        _ => {
            let duration = SystemTime::now()
                .duration_since(start)
                .unwrap_or_default()
                .as_secs_f64();
            log::error!(
                "{}|err|{}|{}|method_not_found|{}",
                request_log_info,
                duration,
                &log_args,
                request.method
            );
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: "Method not found".to_string(),
                    data: None,
                }),
                id: request.id,
            }
        }
    };

    let duration = SystemTime::now()
        .duration_since(start)
        .unwrap_or_default()
        .as_secs_f64();

    if log_args.enable_log() {
        if duration < 1f64 {
            log::info!("{}|ok|{}|{}", request_log_info, duration, &log_args);
        } else {
            log::warn!("{}|ok|{}|{}", request_log_info, duration, &log_args);
        }
    }

    Ok(rpc_response)
}

// 处理 initialize 方法
fn handle_initialize(params: Option<Value>, id: Option<Value>) -> JsonRpcResponse {
    // 从参数中提取 protocolVersion，如果没有则使用默认值
    let protocol_version = params
        .as_ref()
        .and_then(|p| p.get("protocolVersion"))
        .and_then(|v| v.as_str())
        .unwrap_or("2025-03-26"); // 使用默认值作为后备,sse使用: 2024-11-05,mcp: 2025-03-26

    // 返回服务器能力信息
    let result = json!({
        "protocolVersion": protocol_version,
        "capabilities": {
            "experimental": {},
            "prompts": {
                "listChanged": false
            },
            "resources": {
                "subscribe": false,
                "listChanged": false
            },
            "tools": {
                "listChanged": false
            }
        },
        "serverInfo": {
            "name": "r-nacos-mcp-server",
            "version": get_app_version()
        }
    });

    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        result: Some(result),
        error: None,
        id,
    }
}

// 处理 tools/call 方法
async fn handle_tools_call(
    params: Option<Value>,
    id: Option<Value>,
    mcp_server: &Arc<McpServer>,
    app_share_data: &Arc<AppShareData>,
    headers: HashMap<&str, &[u8]>,
    log_args: &mut McpHandleLogArgs,
) -> anyhow::Result<JsonRpcResponse> {
    if let Some(params_value) = params {
        if let (Some(tool_name), Some(args)) = (
            params_value.get("name").and_then(|v| v.as_str()),
            params_value.get("arguments"),
        ) {
            *log_args = McpHandleLogArgs::Arg(format!("tool:{}", tool_name));

            let (tool, url) = match select_tool_and_url(tool_name, &mcp_server, app_share_data)
                .await
            {
                Ok(result) => result,
                Err(error) => {
                    *log_args = McpHandleLogArgs::Arg(format!("tool:{}|select_failed", tool_name));
                    return Err(error);
                }
            };
            let client = &app_share_data.common_client;
            let mut req = match tool.route_rule.convert_type {
                ConvertType::None | ConvertType::Custom => client
                    .request(
                        reqwest::Method::from_bytes(tool.route_rule.method.as_str().as_bytes())?,
                        url.clone(),
                    )
                    .header("content-type", "application/json;charset=UTF-8")
                    .body(serde_json::to_string(args)?),
                ConvertType::JsonToForm => client
                    .request(
                        reqwest::Method::from_bytes(tool.route_rule.method.as_str().as_bytes())?,
                        url.clone(),
                    )
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(serde_urlencoded::to_string(args)?),
                ConvertType::JsonToUrl => {
                    let part = serde_urlencoded::to_string(args)?;
                    let query_url = if url.find("?").is_some() {
                        format!("{}&{}", &url, part)
                    } else {
                        format!("{}?{}", &url, part)
                    };
                    client.request(
                        reqwest::Method::from_bytes(tool.route_rule.method.as_str().as_bytes())?,
                        query_url,
                    )
                }
            };
            let mut user_keys = vec![];
            for (k, v) in tool.route_rule.addition_headers.iter() {
                req = req.header(k, v.as_str());
                user_keys.push(k);
            }
            for (k, v) in headers.iter() {
                if filter_keys(&mut user_keys, k) {
                    continue;
                }
                req = req.header(*k, String::from_utf8_lossy(*v).as_ref());
            }
            #[cfg(feature = "debug")]
            log::info!(
                "headers: {}",
                serde_json::to_string(&headers).unwrap_or_default()
            );
            let res = match req.send().await {
                Ok(response) => response,
                Err(error) => {
                    *log_args = McpHandleLogArgs::Arg(format!(
                        "tool:{}|http_request_failed|{}",
                        tool_name, &error
                    ));
                    return Err(anyhow::anyhow!("HTTP request failed: {}", error));
                }
            };

            let response_status = res.status().as_u16();
            if response_status == 200 {
                let content = match res.text().await {
                    Ok(text) => text,
                    Err(error) => {
                        *log_args = McpHandleLogArgs::Arg(format!(
                            "tool:{}|read_response_failed|{}",
                            tool_name, &error
                        ));
                        return Err(anyhow::anyhow!("Failed to read response: {}", error));
                    }
                };
                *log_args = McpHandleLogArgs::Arg(format!("tool:{}|success", tool_name));
                let result = json!({ "content": [{"type":"text","text":content}]});
                return Ok(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: Some(result),
                    error: None,
                    id,
                });
            } else {
                let content = match res.text().await {
                    Ok(text) => text,
                    Err(error) => {
                        *log_args = McpHandleLogArgs::Arg(format!(
                            "tool:{}|read_error_response_failed|{}",
                            tool_name, &error
                        ));
                        return Err(anyhow::anyhow!("Failed to read error response: {}", error));
                    }
                };
                *log_args = McpHandleLogArgs::Arg(format!(
                    "tool:{}|http_status_{}",
                    tool_name, response_status
                ));
                return Ok(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32000,
                        message: content,
                        data: None,
                    }),
                    id,
                });
            }
        }
    }

    // 如果参数无效或工具不存在，返回错误
    *log_args = McpHandleLogArgs::Arg("invalid_params".to_string());
    Ok(JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        result: None,
        error: Some(JsonRpcError {
            code: -32602,
            message: "Invalid params".to_string(),
            data: None,
        }),
        id,
    })
}

#[inline]
fn filter_keys(user_keys: &Vec<&String>, k: &&str) -> bool {
    for use_key in user_keys.iter() {
        if k.eq_ignore_ascii_case(use_key) {
            return true;
        }
    }
    false
}

async fn select_tool_and_url<'a>(
    tool_name: &str,
    server: &'a Arc<McpServer>,
    app_share_data: &Arc<AppShareData>,
) -> anyhow::Result<(&'a McpTool, String)> {
    for tool in server.release_value.tools.iter() {
        if tool.tool_name.as_str() == tool_name {
            let service_key = ServiceKey::new_by_arc(
                tool.tool_key.namespace.clone(),
                tool.route_rule.service_group.clone(),
                tool.route_rule.service_name.clone(),
            );
            if let Ok(Ok(NamingResult::SelectInstance(instance))) = app_share_data
                .naming_addr
                .send(NamingCmd::SelectOneInstance(service_key))
                .await
            {
                let host = instance.map(|i| (i.ip.clone(), i.port as u16));
                let url = tool.route_rule.build_url(host)?;
                return Ok((tool, url));
            }
        }
    }
    Err(anyhow::anyhow!("mcp server tool not found: {}", tool_name))
}

// 处理 tools/list 方法
fn handle_tools_list(id: Option<Value>, mcp_server: &Arc<McpServer>) -> JsonRpcResponse {
    // 返回可用工具列表
    let tools: Vec<&Arc<ToolFunctionValue>> = mcp_server
        .release_value
        .tools
        .iter()
        .map(|t| &t.spec)
        .collect();

    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        result: Some(json!({ "tools": tools })),
        error: None,
        id,
    }
}

pub async fn mcp_get_handler() -> Result<HttpResponse> {
    Ok(HttpResponse::MethodNotAllowed().body("METHOD_NOT_ALLOWED"))
}

pub async fn mcp_delete_handler() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().body("ok"))
}
