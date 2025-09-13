use crate::common::appdata::AppShareData;
use crate::common::constant::{SEQ_MCP_SERVER_ID, SEQ_MCP_SERVER_VALUE_ID};
use crate::common::model::{ApiResult, PageResult, UserSession};
use crate::common::string_utils::StringUtils;
use crate::console::model::mcp_server_model::{
    McpServerHistoryPublishParams, McpServerHistoryQueryRequest, McpServerParams,
    McpServerQueryRequest, McpServerValueDto,
};
use crate::console::v2::{
    handle_error, handle_mcp_manager_error, handle_not_found_error, handle_param_error,
    handle_system_error, handle_unexpected_response_error,
};
use crate::mcp::model::actor_model::{McpManagerRaftReq, McpManagerReq, McpManagerResult};
use crate::mcp::model::mcp::McpServerDto;
use crate::raft::store::{ClientRequest, ClientResponse};
use crate::sequence::{SequenceRequest, SequenceResult};
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse, Responder};
use std::sync::Arc;

/// 查询McpServer列表
pub async fn query_mcp_server_list(
    _req: HttpRequest,
    request: web::Query<McpServerQueryRequest>,
    appdata: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    // 验证查询参数
    if let Err(err) = request.validate() {
        return handle_param_error(err, "McpServer query parameter validation failed");
    }
    // 转换查询参数
    let query_param = request.to_mcp_query_param();
    // 发送查询请求到MCP Manager
    let cmd = McpManagerReq::QueryServer(query_param);
    match appdata.mcp_manager.send(cmd).await {
        Ok(res) => match res {
            Ok(McpManagerResult::ServerPageInfo(total_count, list)) => {
                HttpResponse::Ok().json(ApiResult::success(Some(PageResult { total_count, list })))
            }
            Ok(_) => handle_unexpected_response_error("MCP Manager query McpServer"),
            Err(err) => handle_mcp_manager_error(err, "query McpServer"),
        },
        Err(err) => handle_system_error(
            format!("Unable to connect to MCP Manager: {}", err),
            "Failed to send query request to MCP Manager",
        ),
    }
}

/// 获取单个McpServer
pub async fn get_mcp_server(
    _req: HttpRequest,
    request: web::Query<McpServerParams>,
    appdata: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    // 验证参数 - 只需要验证ID字段
    if let Err(err) = request.validate_for_delete() {
        return handle_param_error(err, "McpServer ID parameter validation failed");
    }

    let server_id = request.id.unwrap(); // 已经通过validate_for_delete验证，不会为None

    // 发送查询请求到MCP Manager
    let cmd = McpManagerReq::GetServer(server_id);
    match appdata.mcp_manager.send(cmd).await {
        Ok(res) => match res {
            Ok(McpManagerResult::ServerInfo(Some(server))) => {
                log::debug!("Successfully retrieved McpServer with ID: {}", server_id);
                let dto = McpServerDto::new_from(&server);
                HttpResponse::Ok().json(ApiResult::success(Some(dto)))
            }
            Ok(McpManagerResult::ServerInfo(None)) => {
                handle_not_found_error("McpServer", &server_id.to_string())
            }
            Ok(_) => handle_unexpected_response_error("MCP Manager get McpServer"),
            Err(err) => handle_mcp_manager_error(err, "get McpServer"),
        },
        Err(err) => handle_system_error(
            format!("Unable to connect to MCP Manager: {}", err),
            "Failed to send get request to MCP Manager",
        ),
    }
}

/// 新增McpServer
pub async fn add_mcp_server(
    req: HttpRequest,
    appdata: web::Data<Arc<AppShareData>>,
    web::Json(param): web::Json<McpServerParams>,
) -> impl Responder {
    match do_add_mcp_server(req, appdata, param).await {
        Ok(response) => response,
        Err(e) => handle_error(e),
    }
}

async fn do_add_mcp_server(
    req: HttpRequest,
    appdata: web::Data<Arc<AppShareData>>,
    param: McpServerParams,
) -> anyhow::Result<HttpResponse> {
    // 验证参数
    param.validate()?;

    if let Some(server_key) = param.unique_key.as_ref() {
        if !server_key.is_empty() {
            let check_result = appdata
                .mcp_manager
                .send(McpManagerReq::GetServerByKey(Arc::new(
                    server_key.to_string(),
                )))
                .await??;

            if let McpManagerResult::ServerInfo(Some(server)) = check_result {
                return Err(anyhow::anyhow!(
                    "McpServer with key {} already exists, other id: {}",
                    server_key,
                    server.id
                ));
            }
        }
    }

    // 从HttpRequest中提取用户会话信息作为op_user
    let op_user = req
        .extensions()
        .get::<Arc<UserSession>>()
        .map(|session| session.username.clone());

    let server_id = match appdata
        .sequence_manager
        .send(SequenceRequest::GetNextId(SEQ_MCP_SERVER_ID.clone()))
        .await??
    {
        SequenceResult::NextId(id) => id,
        _ => return Err(anyhow::anyhow!("Failed to get server id")),
    };

    let server_value_id = match appdata
        .sequence_manager
        .send(SequenceRequest::GetNextId(SEQ_MCP_SERVER_VALUE_ID.clone()))
        .await??
    {
        SequenceResult::NextId(id) => id,
        _ => return Err(anyhow::anyhow!("Failed to get server value id")),
    };

    let new_value_id = match appdata
        .sequence_manager
        .send(SequenceRequest::GetNextId(SEQ_MCP_SERVER_VALUE_ID.clone()))
        .await??
    {
        SequenceResult::NextId(id) => id,
        _ => return Err(anyhow::anyhow!("Failed to get new value id")),
    };

    // 转换为McpServerParam
    let mut server_param = param.to_mcp_server_param(op_user.clone());
    server_param.id = server_id;
    server_param.value_id = server_value_id;
    server_param.publish_value_id = Some(new_value_id);
    if StringUtils::is_option_empty_arc(&server_param.unique_key) {
        server_param.unique_key = Some(server_param.build_unique_key());
    }

    // 构建McpManagerRaftReq::AddServer请求
    let raft_req = McpManagerRaftReq::AddServer(server_param);
    let client_req = ClientRequest::McpReq { req: raft_req };

    // 通过RaftRequestRoute.request发送写入请求
    let response = appdata.raft_request_route.request(client_req).await?;

    match response {
        ClientResponse::McpResp { resp: _ } => {
            Ok(HttpResponse::Ok().json(ApiResult::success(Some(server_id))))
        }
        _ => Err(anyhow::anyhow!(
            "Unexpected response from Raft add McpServer"
        )),
    }
}

/// 更新McpServer
pub async fn update_mcp_server(
    req: HttpRequest,
    appdata: web::Data<Arc<AppShareData>>,
    web::Json(param): web::Json<McpServerParams>,
) -> impl Responder {
    match do_update_mcp_server(req, appdata, param).await {
        Ok(response) => response,
        Err(e) => handle_error(e),
    }
}

async fn do_update_mcp_server(
    req: HttpRequest,
    appdata: web::Data<Arc<AppShareData>>,
    param: McpServerParams,
) -> anyhow::Result<HttpResponse> {
    // 验证参数 - 使用更新专用的验证方法
    param.validate_for_update()?;

    let server_id = param.id.unwrap(); // 已经通过validate_for_update验证，不会为None

    // 从HttpRequest中提取用户会话信息作为op_user
    let op_user = req
        .extensions()
        .get::<Arc<UserSession>>()
        .map(|session| session.username.clone());

    // 首先检查McpServer是否存在
    let check_cmd = McpManagerReq::GetServer(server_id);
    let check_result = appdata.mcp_manager.send(check_cmd).await??;

    match check_result {
        McpManagerResult::ServerInfo(Some(_)) => {}
        _ => {
            return Err(anyhow::anyhow!("McpServer not found: {}", server_id));
        }
    };

    // 转换为McpServerParam，支持部分字段更新
    let server_param = param.to_mcp_server_param(op_user.clone());

    // 构建McpManagerRaftReq::UpdateServer请求
    let raft_req = McpManagerRaftReq::UpdateServer(server_param);
    let client_req = ClientRequest::McpReq { req: raft_req };

    // 通过RaftRequestRoute.request发送更新请求
    let response = appdata.raft_request_route.request(client_req).await?;

    match response {
        ClientResponse::McpResp { resp: _ } => {
            Ok(HttpResponse::Ok().json(ApiResult::success(Some(true))))
        }
        _ => Err(anyhow::anyhow!(
            "Unexpected response from Raft update McpServer"
        )),
    }
}

/// 删除McpServer
pub async fn remove_mcp_server(
    req: HttpRequest,
    appdata: web::Data<Arc<AppShareData>>,
    web::Json(param): web::Json<McpServerParams>,
) -> impl Responder {
    match do_remove_mcp_server(req, appdata, param).await {
        Ok(response) => response,
        Err(e) => handle_error(e),
    }
}

async fn do_remove_mcp_server(
    _req: HttpRequest,
    appdata: web::Data<Arc<AppShareData>>,
    param: McpServerParams,
) -> anyhow::Result<HttpResponse> {
    // 验证参数 - 只需要验证ID字段
    param.validate_for_delete()?;

    let server_id = param.id.unwrap(); // 已经通过validate_for_delete验证，不会为None

    // 首先检查McpServer是否存在
    let cmd = McpManagerReq::GetServer(server_id);
    let check_result = appdata.mcp_manager.send(cmd).await??;

    match check_result {
        McpManagerResult::ServerInfo(Some(_)) => {}
        _ => {
            return Err(anyhow::anyhow!("McpServer not found: {}", server_id));
        }
    };

    // 构建McpManagerRaftReq::RemoveServer请求
    let raft_req = McpManagerRaftReq::RemoveServer(server_id);
    let client_req = ClientRequest::McpReq { req: raft_req };

    // 通过RaftRequestRoute.request发送删除请求
    let response = appdata.raft_request_route.request(client_req).await?;

    match response {
        ClientResponse::Success => Ok(HttpResponse::Ok().json(ApiResult::success(Some(true)))),
        ClientResponse::McpResp { resp: _ } => {
            Ok(HttpResponse::Ok().json(ApiResult::success(Some(true))))
        }
        _ => Err(anyhow::anyhow!(
            "Unexpected response from Raft remove McpServer"
        )),
    }
}

/// 查询McpServer历史版本
pub async fn query_mcp_server_history(
    _req: HttpRequest,
    request: web::Query<McpServerHistoryQueryRequest>,
    appdata: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    // 验证查询参数
    if let Err(err) = request.validate() {
        return handle_param_error(err, "McpServer history query parameter validation failed");
    }

    let server_id = request.id;
    let (offset, limit) = request.get_pagination();

    // 首先检查McpServer是否存在
    let check_cmd = McpManagerReq::GetServer(server_id);
    match appdata.mcp_manager.send(check_cmd).await {
        Ok(res) => match res {
            Ok(McpManagerResult::ServerInfo(Some(_))) => {}
            _ => {
                return handle_not_found_error("McpServer", &server_id.to_string());
            }
        },
        Err(err) => {
            return handle_system_error(
                format!("Unable to connect to MCP Manager: {}", err),
                "Failed to send check request to MCP Manager",
            );
        }
    }

    // 发送历史版本查询请求到MCP Manager
    let cmd = McpManagerReq::QueryServerHistory(
        server_id,
        offset,
        limit,
        request.start_time,
        request.end_time,
    );
    match appdata.mcp_manager.send(cmd).await {
        Ok(res) => match res {
            Ok(McpManagerResult::ServerHistoryPageInfo(total_count, history_list)) => {
                // 转换为DTO格式
                let dto_list: Vec<McpServerValueDto> = history_list
                    .iter()
                    .map(|history| McpServerValueDto::from_value(history))
                    .collect();

                HttpResponse::Ok().json(ApiResult::success(Some(PageResult {
                    total_count,
                    list: dto_list,
                })))
            }
            Ok(_) => handle_unexpected_response_error("MCP Manager query McpServer history"),
            Err(err) => handle_mcp_manager_error(err, "query McpServer history"),
        },
        Err(err) => handle_system_error(
            format!("Unable to connect to MCP Manager: {}", err),
            "Failed to send history query request to MCP Manager",
        ),
    }
}

/// 发布当前版本
pub async fn publish_current_mcp_server(
    req: HttpRequest,
    appdata: web::Data<Arc<AppShareData>>,
    web::Json(param): web::Json<McpServerParams>,
) -> impl Responder {
    match do_publish_current_mcp_server(req, appdata, param).await {
        Ok(response) => response,
        Err(e) => handle_error(e),
    }
}

async fn do_publish_current_mcp_server(
    _req: HttpRequest,
    appdata: web::Data<Arc<AppShareData>>,
    param: McpServerParams,
) -> anyhow::Result<HttpResponse> {
    // 验证参数 - 只需要验证ID字段
    param.validate_for_delete()?;

    let server_id = param.id.unwrap(); // 已经通过validate_for_delete验证，不会为None

    // 首先检查McpServer是否存在
    let check_cmd = McpManagerReq::GetServer(server_id);
    let check_result = appdata.mcp_manager.send(check_cmd).await??;

    match check_result {
        McpManagerResult::ServerInfo(Some(_)) => {}
        _ => {
            return Err(anyhow::anyhow!("McpServer not found: {}", server_id));
        }
    };

    let server_value_id = match appdata
        .sequence_manager
        .send(SequenceRequest::GetNextId(SEQ_MCP_SERVER_VALUE_ID.clone()))
        .await??
    {
        SequenceResult::NextId(id) => id,
        _ => return Err(anyhow::anyhow!("Failed to get server value id")),
    };

    // 构建McpManagerRaftReq::PublishCurrentServer请求
    let raft_req = McpManagerRaftReq::PublishCurrentServer(server_id, server_value_id);
    let client_req = ClientRequest::McpReq { req: raft_req };

    // 通过RaftRequestRoute.request发送版本发布请求
    let response = appdata.raft_request_route.request(client_req).await?;

    match response {
        ClientResponse::Success => Ok(HttpResponse::Ok().json(ApiResult::success(Some(true)))),
        ClientResponse::McpResp { resp: _ } => {
            Ok(HttpResponse::Ok().json(ApiResult::success(Some(true))))
        }
        _ => Err(anyhow::anyhow!(
            "Unexpected response from Raft publish current McpServer version"
        )),
    }
}

/// 发布历史版本
pub async fn publish_history_mcp_server(
    req: HttpRequest,
    appdata: web::Data<Arc<AppShareData>>,
    web::Json(param): web::Json<McpServerHistoryPublishParams>,
) -> impl Responder {
    match do_publish_history_mcp_server(req, appdata, param).await {
        Ok(response) => response,
        Err(e) => handle_error(e),
    }
}

async fn do_publish_history_mcp_server(
    _req: HttpRequest,
    appdata: web::Data<Arc<AppShareData>>,
    param: McpServerHistoryPublishParams,
) -> anyhow::Result<HttpResponse> {
    // 验证参数
    param.validate()?;

    let server_id = param.id;
    let history_value_id = param.history_value_id;

    // 首先检查McpServer是否存在
    let check_cmd = McpManagerReq::GetServer(server_id);
    match appdata.mcp_manager.send(check_cmd).await?? {
        McpManagerResult::ServerInfo(Some(_)) => {}
        _ => {
            return Err(anyhow::anyhow!("McpServer not found: {}", server_id));
        }
    };

    // 构建McpManagerRaftReq::PublishHistoryServer请求
    let raft_req = McpManagerRaftReq::PublishHistoryServer(server_id, history_value_id);
    let client_req = ClientRequest::McpReq { req: raft_req };

    // 通过RaftRequestRoute.request发送历史版本回滚发布请求
    let response = appdata.raft_request_route.request(client_req).await?;

    match response {
        ClientResponse::Success | ClientResponse::McpResp { resp: _ } => {
            Ok(HttpResponse::Ok().json(ApiResult::success(Some(true))))
        }
        _ => Err(anyhow::anyhow!(
            "Unexpected response from Raft publish history McpServer version"
        )),
    }
}
