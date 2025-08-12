use crate::common::appdata::AppShareData;
use crate::common::model::{ApiResult, PageResult, UserSession};
use crate::console::model::mcp_tool_spec_model::{ToolSpecParams, ToolSpecQueryRequest};
use crate::console::v2::{
    handle_mcp_manager_error, handle_not_found_error, handle_param_error, handle_raft_error,
    handle_system_error, handle_unexpected_response_error,
};
use crate::mcp::model::actor_model::{
    McpManagerRaftReq, McpManagerReq, McpManagerResult, ToolSpecDto,
};
use crate::raft::store::{ClientRequest, ClientResponse};
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse, Responder};
use std::sync::Arc;

/// 查询ToolSpec列表
pub async fn query_tool_spec_list(
    _req: HttpRequest,
    request: web::Query<ToolSpecQueryRequest>,
    appdata: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    // 验证查询参数
    if let Err(err) = request.validate() {
        return handle_param_error(err, "ToolSpec query parameter validation failed");
    }

    // 转换查询参数
    let query_param = request.to_query_param();

    log::debug!(
        "Query ToolSpec list: offset={}, limit={}, namespace_filter={:?}, group_filter={:?}, tool_name_filter={:?}",
        query_param.offset,
        query_param.limit,
        query_param.namespace_id,
        query_param.group_filter,
        query_param.tool_name_filter
    );

    // 发送查询请求到MCP Manager
    let cmd = McpManagerReq::QueryToolSpec(query_param);
    match appdata.mcp_manager.send(cmd).await {
        Ok(res) => match res {
            Ok(McpManagerResult::ToolSpecPageInfo(total_count, list)) => {
                log::debug!(
                    "Successfully queried {} ToolSpec records, total count: {}",
                    list.len(),
                    total_count
                );
                HttpResponse::Ok().json(ApiResult::success(Some(PageResult { total_count, list })))
            }
            Ok(_) => handle_unexpected_response_error("MCP Manager query ToolSpec"),
            Err(err) => handle_mcp_manager_error(err, "query ToolSpec"),
        },
        Err(err) => handle_system_error(
            format!("Unable to connect to MCP Manager: {}", err),
            "Failed to send query request to MCP Manager",
        ),
    }
}

/// 获取单个ToolSpec
pub async fn get_tool_spec(
    _req: HttpRequest,
    web::Query(param): web::Query<ToolSpecParams>,
    appdata: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    // 验证参数
    if let Err(err) = param.validate() {
        return handle_param_error(err, "ToolSpec get parameter validation failed");
    }

    // 构建ToolKey
    let tool_key = param.to_tool_key();

    log::debug!(
        "Get ToolSpec: namespace={}, group={}, tool_name={}",
        tool_key.namespace,
        tool_key.group,
        tool_key.tool_name
    );

    // 发送获取请求到MCP Manager
    let cmd = McpManagerReq::GetToolSpec(tool_key);
    match appdata.mcp_manager.send(cmd).await {
        Ok(res) => match res {
            Ok(McpManagerResult::ToolSpecInfo(Some(tool_spec))) => {
                log::debug!("Successfully retrieved ToolSpec: {:?}", tool_spec.key);
                let dto = ToolSpecDto::new_from(&tool_spec);
                HttpResponse::Ok().json(ApiResult::success(Some(dto)))
            }
            Ok(McpManagerResult::ToolSpecInfo(None)) => {
                handle_not_found_error("ToolSpec", &format!("{:?}", param))
            }
            Ok(_) => handle_unexpected_response_error("MCP Manager get ToolSpec"),
            Err(err) => handle_mcp_manager_error(err, "get ToolSpec"),
        },
        Err(err) => handle_system_error(
            format!("Unable to connect to MCP Manager: {}", err),
            "Failed to send get request to MCP Manager",
        ),
    }
}

/// 创建或更新ToolSpec
pub async fn add_or_update_tool_spec(
    req: HttpRequest,
    appdata: web::Data<Arc<AppShareData>>,
    web::Json(param): web::Json<ToolSpecParams>,
) -> impl Responder {
    // 验证参数
    if let Err(err) = param.validate() {
        return handle_param_error(err, "ToolSpec create/update parameter validation failed");
    }

    // 从HttpRequest中提取用户会话信息作为op_user
    let op_user = req
        .extensions()
        .get::<Arc<UserSession>>()
        .map(|session| session.username.clone());

    log::debug!(
        "Create/Update ToolSpec: namespace={}, group={}, tool_name={}, op_user={:?}",
        param.namespace,
        param.group,
        param.tool_name,
        op_user
    );

    // 转换为ToolSpecParam
    let tool_spec_param = param.to_tool_spec_param(op_user);

    // 构建McpManagerRaftReq::UpdateToolSpec请求
    let raft_req = McpManagerRaftReq::UpdateToolSpec(tool_spec_param);
    let client_req = ClientRequest::McpReq { req: raft_req };

    // 通过RaftRequestRoute.request发送写入请求
    match appdata.raft_request_route.request(client_req).await {
        Ok(response) => match response {
            ClientResponse::Success => {
                log::debug!(
                    "Successfully created/updated ToolSpec: {:?}",
                    param.to_tool_key()
                );
                HttpResponse::Ok().json(ApiResult::success(Some(true)))
            }
            ClientResponse::McpResp { resp } => {
                log::debug!("MCP Raft response: {:?}", resp);
                HttpResponse::Ok().json(ApiResult::success(Some(true)))
            }
            _ => handle_unexpected_response_error("Raft create/update ToolSpec"),
        },
        Err(err) => handle_raft_error(err, "create/update ToolSpec"),
    }
}

/// 删除ToolSpec
pub async fn remove_tool_spec(
    req: HttpRequest,
    appdata: web::Data<Arc<AppShareData>>,
    web::Json(param): web::Json<ToolSpecParams>,
) -> impl Responder {
    // 验证删除参数的有效性（只需要验证key字段）
    if let Err(err) = param.validate_for_delete() {
        return handle_param_error(err, "ToolSpec delete parameter validation failed");
    }

    // 从HttpRequest中提取用户会话信息作为op_user
    let op_user = req
        .extensions()
        .get::<Arc<UserSession>>()
        .map(|session| session.username.clone());

    log::debug!(
        "Delete ToolSpec: namespace={}, group={}, tool_name={}, op_user={:?}",
        param.namespace,
        param.group,
        param.tool_name,
        op_user
    );

    // 构建ToolKey
    let tool_key = param.to_tool_key();

    // 构建McpManagerRaftReq::RemoveToolSpec请求
    let raft_req = McpManagerRaftReq::RemoveToolSpec(tool_key.clone());
    let client_req = ClientRequest::McpReq { req: raft_req };

    // 通过RaftRequestRoute.request发送删除请求
    match appdata.raft_request_route.request(client_req).await {
        Ok(response) => match response {
            ClientResponse::Success => {
                log::debug!("Successfully deleted ToolSpec: {:?}", tool_key);
                HttpResponse::Ok().json(ApiResult::success(Some(true)))
            }
            ClientResponse::McpResp { resp: _ } => {
                log::debug!("Successfully deleted ToolSpec: {:?}", tool_key);
                HttpResponse::Ok().json(ApiResult::success(Some(true)))
            }
            _ => handle_unexpected_response_error("Raft delete ToolSpec"),
        },
        Err(err) => {
            // 检查是否是ToolSpec不存在的错误
            let error_msg = err.to_string();
            if error_msg.contains("not found") || error_msg.contains("does not exist") {
                handle_not_found_error("ToolSpec", &format!("{:?}", tool_key))
            } else {
                handle_raft_error(err, "delete ToolSpec")
            }
        }
    }
}
