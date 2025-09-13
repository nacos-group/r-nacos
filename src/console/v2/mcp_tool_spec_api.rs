use crate::common::appdata::AppShareData;
use crate::common::constant::SEQ_TOOL_SPEC_VERSION;
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
use crate::sequence::{SequenceRequest, SequenceResult};
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
    // 发送查询请求到MCP Manager
    let cmd = McpManagerReq::QueryToolSpec(query_param);
    match appdata.mcp_manager.send(cmd).await {
        Ok(res) => match res {
            Ok(McpManagerResult::ToolSpecPageInfo(total_count, list)) => {
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

    // 发送获取请求到MCP Manager
    let cmd = McpManagerReq::GetToolSpec(tool_key);
    match appdata.mcp_manager.send(cmd).await {
        Ok(res) => match res {
            Ok(McpManagerResult::ToolSpecInfo(Some(tool_spec))) => {
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

    let version = if let Ok(Ok(SequenceResult::NextId(id))) = appdata
        .sequence_manager
        .send(SequenceRequest::GetNextId(SEQ_TOOL_SPEC_VERSION.clone()))
        .await
    {
        id
    } else {
        return handle_system_error(
            "Unable to get next id from SequenceManager".to_string(),
            "Failed to get next id from SequenceManager",
        );
    };
    // 转换为ToolSpecParam
    let mut tool_spec_param = param.to_tool_spec_param(op_user);
    tool_spec_param.version = version;

    // 构建McpManagerRaftReq::UpdateToolSpec请求
    let raft_req = McpManagerRaftReq::UpdateToolSpec(tool_spec_param);
    let client_req = ClientRequest::McpReq { req: raft_req };

    // 通过RaftRequestRoute.request发送写入请求
    match appdata.raft_request_route.request(client_req).await {
        Ok(response) => match response {
            ClientResponse::Success => HttpResponse::Ok().json(ApiResult::success(Some(true))),
            ClientResponse::McpResp { resp: _ } => {
                HttpResponse::Ok().json(ApiResult::success(Some(true)))
            }
            _ => handle_unexpected_response_error("Raft create/update ToolSpec"),
        },
        Err(err) => handle_raft_error(err, "create/update ToolSpec"),
    }
}

/// 批量创建或更新ToolSpec
pub async fn update_tool_specs(
    req: HttpRequest,
    appdata: web::Data<Arc<AppShareData>>,
    web::Json(params): web::Json<Vec<ToolSpecParams>>,
) -> impl Responder {
    // 验证参数列表不为空
    if params.is_empty() {
        return handle_param_error(
            anyhow::anyhow!("ToolSpec参数列表不能为空"),
            "ToolSpec batch update parameter validation failed",
        );
    }

    // 验证每个参数
    for (index, param) in params.iter().enumerate() {
        if let Err(err) = param.validate() {
            return handle_param_error(
                anyhow::anyhow!("第{}个参数验证失败: {}", index + 1, err),
                "ToolSpec batch update parameter validation failed",
            );
        }
    }

    // 从HttpRequest中提取用户会话信息作为op_user
    let op_user = req
        .extensions()
        .get::<Arc<UserSession>>()
        .map(|session| session.username.clone());

    // 一次性获取所需数量的版本号范围
    let mut seq_range = if let Ok(Ok(SequenceResult::Range(range))) = appdata
        .sequence_manager
        .send(SequenceRequest::GetDirectRange(
            SEQ_TOOL_SPEC_VERSION.clone(),
            params.len() as u64,
        ))
        .await
    {
        range
    } else {
        return handle_system_error(
            "Unable to get id range from SequenceManager".to_string(),
            "Failed to get id range from SequenceManager",
        );
    };

    // 转换为ToolSpecParam列表，为每个参数分配版本号
    let mut tool_spec_params = Vec::new();
    for param in params.iter() {
        let version = if let Some(id) = seq_range.next_id() {
            id
        } else {
            return handle_system_error(
                "Insufficient version IDs in range".to_string(),
                "Failed to get version ID from range",
            );
        };

        let mut tool_spec_param = param.to_tool_spec_param(op_user.clone());
        tool_spec_param.version = version;
        tool_spec_params.push(tool_spec_param);
    }

    // 构建McpManagerRaftReq::UpdateToolSpecList请求
    let raft_req = McpManagerRaftReq::UpdateToolSpecList(tool_spec_params);
    let client_req = ClientRequest::McpReq { req: raft_req };

    // 通过RaftRequestRoute.request发送写入请求
    match appdata.raft_request_route.request(client_req).await {
        Ok(response) => match response {
            ClientResponse::Success => HttpResponse::Ok().json(ApiResult::success(Some(true))),
            ClientResponse::McpResp { resp: _ } => {
                HttpResponse::Ok().json(ApiResult::success(Some(true)))
            }
            _ => handle_unexpected_response_error("Raft batch create/update ToolSpec"),
        },
        Err(err) => handle_raft_error(err, "batch create/update ToolSpec"),
    }
}

/// 删除ToolSpec
pub async fn remove_tool_spec(
    _req: HttpRequest,
    appdata: web::Data<Arc<AppShareData>>,
    web::Json(param): web::Json<ToolSpecParams>,
) -> impl Responder {
    // 验证删除参数的有效性（只需要验证key字段）
    if let Err(err) = param.validate_for_delete() {
        return handle_param_error(err, "ToolSpec delete parameter validation failed");
    }

    // 构建ToolKey
    let tool_key = param.to_tool_key();

    // 构建McpManagerRaftReq::RemoveToolSpec请求
    let raft_req = McpManagerRaftReq::RemoveToolSpec(tool_key.clone());
    let client_req = ClientRequest::McpReq { req: raft_req };

    // 通过RaftRequestRoute.request发送删除请求
    match appdata.raft_request_route.request(client_req).await {
        Ok(response) => match response {
            ClientResponse::Success => HttpResponse::Ok().json(ApiResult::success(Some(true))),
            ClientResponse::McpResp { resp: _ } => {
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
