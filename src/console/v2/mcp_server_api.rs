use crate::common::appdata::AppShareData;
use crate::common::constant::{SEQ_MCP_SERVER_ID, SEQ_MCP_SERVER_VALUE_ID};
use crate::common::model::{ApiResult, PageResult, UserSession};
use crate::common::string_utils::StringUtils;
use crate::console::model::mcp_server_model::{
    McpServerHistoryPublishParams, McpServerHistoryQueryRequest, McpServerParams,
    McpServerQueryRequest, McpServerValueDto,
};
use crate::console::v2::{
    handle_mcp_manager_error, handle_not_found_error, handle_param_error, handle_raft_error,
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

    log::debug!(
        "Query McpServer list: offset={}, limit={}, namespace_filter={:?}, name_filter={:?}",
        query_param.offset,
        query_param.limit,
        query_param.namespace_id,
        query_param.name_filter
    );

    // 发送查询请求到MCP Manager
    let cmd = McpManagerReq::QueryServer(query_param);
    match appdata.mcp_manager.send(cmd).await {
        Ok(res) => match res {
            Ok(McpManagerResult::ServerPageInfo(total_count, list)) => {
                log::debug!(
                    "Successfully queried {} McpServer records, total count: {}",
                    list.len(),
                    total_count
                );
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

    log::debug!("Get McpServer with ID: {}", server_id);

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
    // 验证参数
    if let Err(err) = param.validate() {
        return handle_param_error(err, "McpServer add parameter validation failed");
    }

    // 从HttpRequest中提取用户会话信息作为op_user
    let op_user = req
        .extensions()
        .get::<Arc<UserSession>>()
        .map(|session| session.username.clone());

    let server_id = if let Ok(Ok(SequenceResult::NextId(id))) = appdata
        .sequence_manager
        .send(SequenceRequest::GetNextId(SEQ_MCP_SERVER_ID.clone()))
        .await
    {
        id
    } else {
        return handle_system_error(
            "Unable to get next id from SequenceManager".to_string(),
            "Failed to get next id from SequenceManager",
        );
    };
    let server_value_id = if let Ok(Ok(SequenceResult::NextId(id))) = appdata
        .sequence_manager
        .send(SequenceRequest::GetNextId(SEQ_MCP_SERVER_VALUE_ID.clone()))
        .await
    {
        id
    } else {
        return handle_system_error(
            "Unable to get next id from SequenceManager".to_string(),
            "Failed to get next id from SequenceManager",
        );
    };
    let new_value_id = if let Ok(Ok(SequenceResult::NextId(id))) = appdata
        .sequence_manager
        .send(SequenceRequest::GetNextId(SEQ_MCP_SERVER_VALUE_ID.clone()))
        .await
    {
        id
    } else {
        return handle_system_error(
            "Unable to get next id from SequenceManager".to_string(),
            "Failed to get next id from SequenceManager",
        );
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
    match appdata.raft_request_route.request(client_req).await {
        Ok(response) => match response {
            ClientResponse::McpResp { resp } => {
                log::debug!("MCP Raft add server response: {:?}", resp);
                HttpResponse::Ok().json(ApiResult::success(Some(server_id)))
            }
            _ => handle_unexpected_response_error("Raft add McpServer"),
        },
        Err(err) => handle_raft_error(err, "add McpServer"),
    }
}

/// 更新McpServer
pub async fn update_mcp_server(
    req: HttpRequest,
    appdata: web::Data<Arc<AppShareData>>,
    web::Json(param): web::Json<McpServerParams>,
) -> impl Responder {
    // 验证参数 - 使用更新专用的验证方法
    if let Err(err) = param.validate_for_update() {
        return handle_param_error(err, "McpServer update parameter validation failed");
    }

    let server_id = param.id.unwrap(); // 已经通过validate_for_update验证，不会为None

    // 从HttpRequest中提取用户会话信息作为op_user
    let op_user = req
        .extensions()
        .get::<Arc<UserSession>>()
        .map(|session| session.username.clone());

    // 首先检查McpServer是否存在
    let check_cmd = McpManagerReq::GetServer(server_id);
    match appdata.mcp_manager.send(check_cmd).await {
        Ok(res) => match res {
            Ok(McpManagerResult::ServerInfo(Some(existing_server))) => {
                log::debug!(
                    "Found existing McpServer to update: id={}, name={:?}, namespace={:?}",
                    server_id,
                    existing_server.name,
                    existing_server.namespace
                );

                // 转换为McpServerParam，支持部分字段更新
                let server_param = param.to_mcp_server_param(op_user.clone());

                // 构建McpManagerRaftReq::UpdateServer请求
                let raft_req = McpManagerRaftReq::UpdateServer(server_param);
                let client_req = ClientRequest::McpReq { req: raft_req };

                // 通过RaftRequestRoute.request发送更新请求
                match appdata.raft_request_route.request(client_req).await {
                    Ok(response) => match response {
                        ClientResponse::McpResp { resp } => {
                            log::debug!("MCP Raft update server response: {:?}", resp);
                            log::info!(
                                "Successfully updated McpServer: id={}, name={:?}, namespace={:?}, op_user={:?}",
                                server_id,
                                param.name,
                                param.namespace,
                                op_user
                            );
                            HttpResponse::Ok().json(ApiResult::success(Some(true)))
                        }
                        _ => {
                            log::error!(
                                "Unexpected response when updating McpServer: id={}, op_user={:?}",
                                server_id,
                                op_user
                            );
                            handle_unexpected_response_error("Raft update McpServer")
                        }
                    },
                    Err(err) => {
                        log::error!(
                            "Failed to update McpServer: id={}, op_user={:?}, error={}",
                            server_id,
                            op_user,
                            err
                        );
                        handle_raft_error(err, "update McpServer")
                    }
                }
            }
            Ok(McpManagerResult::ServerInfo(None)) => {
                log::warn!(
                    "Attempted to update non-existent McpServer: id={}, op_user={:?}",
                    server_id,
                    op_user
                );
                handle_not_found_error("McpServer", &server_id.to_string())
            }
            Ok(_) => {
                log::error!(
                    "Unexpected response type when checking McpServer existence for update: id={}, op_user={:?}",
                    server_id,
                    op_user
                );
                handle_unexpected_response_error("MCP Manager get McpServer for update")
            }
            Err(err) => {
                log::error!(
                    "Failed to check McpServer existence before update: id={}, op_user={:?}, error={}",
                    server_id,
                    op_user,
                    err
                );
                handle_mcp_manager_error(err, "check McpServer existence for update")
            }
        },
        Err(err) => {
            log::error!(
                "Unable to connect to MCP Manager for update check: id={}, op_user={:?}, error={}",
                server_id,
                op_user,
                err
            );
            handle_system_error(
                format!("Unable to connect to MCP Manager: {}", err),
                "Failed to send update check request to MCP Manager",
            )
        }
    }
}

/// 删除McpServer
pub async fn remove_mcp_server(
    req: HttpRequest,
    appdata: web::Data<Arc<AppShareData>>,
    web::Json(param): web::Json<McpServerParams>,
) -> impl Responder {
    // 验证参数 - 只需要验证ID字段
    if let Err(err) = param.validate_for_delete() {
        return handle_param_error(err, "McpServer delete parameter validation failed");
    }

    let server_id = param.id.unwrap(); // 已经通过validate_for_delete验证，不会为None

    // 从HttpRequest中提取用户会话信息作为op_user
    let op_user = req
        .extensions()
        .get::<Arc<UserSession>>()
        .map(|session| session.username.clone())
        .unwrap_or_else(|| Arc::new("system".to_string()));

    log::debug!("Remove McpServer: id={}, op_user={:?}", server_id, op_user);

    // 首先检查McpServer是否存在
    let cmd = McpManagerReq::GetServer(server_id);
    match appdata.mcp_manager.send(cmd).await {
        Ok(res) => match res {
            Ok(McpManagerResult::ServerInfo(Some(server))) => {
                log::debug!(
                    "Found McpServer to delete: id={}, name={:?}, namespace={:?}",
                    server_id,
                    server.name,
                    server.namespace
                );

                // 记录删除操作的审计日志
                log::info!(
                    "User {} is deleting McpServer: id={}, name={:?}, namespace={:?}",
                    op_user,
                    server_id,
                    server.name,
                    server.namespace
                );

                // 构建McpManagerRaftReq::RemoveServer请求
                let raft_req = McpManagerRaftReq::RemoveServer(server_id);
                let client_req = ClientRequest::McpReq { req: raft_req };

                // 通过RaftRequestRoute.request发送删除请求
                match appdata.raft_request_route.request(client_req).await {
                    Ok(response) => match response {
                        ClientResponse::Success => {
                            log::info!(
                                "Successfully deleted McpServer: id={}, name={:?}, namespace={:?}, op_user={}",
                                server_id,
                                server.name,
                                server.namespace,
                                op_user
                            );
                            HttpResponse::Ok().json(ApiResult::success(Some(true)))
                        }
                        ClientResponse::McpResp { resp } => {
                            log::debug!("MCP Raft remove server response: {:?}", resp);
                            log::info!(
                                "Successfully deleted McpServer: id={}, name={:?}, namespace={:?}, op_user={}",
                                server_id,
                                server.name,
                                server.namespace,
                                op_user
                            );
                            HttpResponse::Ok().json(ApiResult::success(Some(true)))
                        }
                        _ => {
                            log::error!(
                                "Unexpected response when deleting McpServer: id={}, op_user={}",
                                server_id,
                                op_user
                            );
                            handle_unexpected_response_error("Raft remove McpServer")
                        }
                    },
                    Err(err) => {
                        log::error!(
                            "Failed to delete McpServer: id={}, op_user={}, error={}",
                            server_id,
                            op_user,
                            err
                        );
                        handle_raft_error(err, "remove McpServer")
                    }
                }
            }
            Ok(McpManagerResult::ServerInfo(None)) => {
                log::warn!(
                    "Attempted to delete non-existent McpServer: id={}, op_user={}",
                    server_id,
                    op_user
                );
                handle_not_found_error("McpServer", &server_id.to_string())
            }
            Ok(_) => {
                log::error!(
                    "Unexpected response type when checking McpServer existence: id={}, op_user={}",
                    server_id,
                    op_user
                );
                handle_unexpected_response_error("MCP Manager get McpServer for deletion")
            }
            Err(err) => {
                log::error!(
                    "Failed to check McpServer existence before deletion: id={}, op_user={}, error={}",
                    server_id,
                    op_user,
                    err
                );
                handle_mcp_manager_error(err, "check McpServer existence for deletion")
            }
        },
        Err(err) => {
            log::error!(
                "Unable to connect to MCP Manager for deletion check: id={}, op_user={}, error={}",
                server_id,
                op_user,
                err
            );
            handle_system_error(
                format!("Unable to connect to MCP Manager: {}", err),
                "Failed to send deletion check request to MCP Manager",
            )
        }
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

    log::debug!(
        "Query McpServer history: id={}, offset={}, limit={}, start_time={:?}, end_time={:?}",
        server_id,
        offset,
        limit,
        request.start_time,
        request.end_time
    );

    // 首先检查McpServer是否存在
    let check_cmd = McpManagerReq::GetServer(server_id);
    match appdata.mcp_manager.send(check_cmd).await {
        Ok(res) => match res {
            Ok(McpManagerResult::ServerInfo(Some(_))) => {
                // McpServer存在，继续查询历史版本
                log::debug!("McpServer {} exists, querying history", server_id);
            }
            Ok(McpManagerResult::ServerInfo(None)) => {
                log::warn!("McpServer {} not found when querying history", server_id);
                return handle_not_found_error("McpServer", &server_id.to_string());
            }
            Ok(_) => {
                return handle_unexpected_response_error("MCP Manager check McpServer existence");
            }
            Err(err) => {
                return handle_mcp_manager_error(
                    err,
                    "check McpServer existence for history query",
                );
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
                log::debug!(
                    "Successfully queried {} McpServer history records for server {}, total count: {}",
                    history_list.len(),
                    server_id,
                    total_count
                );

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
    // 验证参数 - 只需要验证ID字段
    if let Err(err) = param.validate_for_delete() {
        return handle_param_error(err, "McpServer publish parameter validation failed");
    }

    let server_id = param.id.unwrap(); // 已经通过validate_for_delete验证，不会为None

    // 从HttpRequest中提取用户会话信息作为op_user
    let op_user = req
        .extensions()
        .get::<Arc<UserSession>>()
        .map(|session| session.username.clone())
        .unwrap_or_else(|| Arc::new("system".to_string()));

    log::debug!(
        "Publish current McpServer version: id={}, op_user={:?}",
        server_id,
        op_user
    );

    // 首先检查McpServer是否存在
    let check_cmd = McpManagerReq::GetServer(server_id);
    match appdata.mcp_manager.send(check_cmd).await {
        Ok(res) => match res {
            Ok(McpManagerResult::ServerInfo(Some(server))) => {
                log::debug!(
                    "Found McpServer to publish: id={}, name={:?}, namespace={:?}",
                    server_id,
                    server.name,
                    server.namespace
                );

                // 记录版本发布的操作日志
                log::info!(
                    "User {} is publishing current version of McpServer: id={}, name={:?}, namespace={:?}",
                    op_user,
                    server_id,
                    server.name,
                    server.namespace
                );

                let server_value_id = if let Ok(Ok(SequenceResult::NextId(id))) = appdata
                    .sequence_manager
                    .send(SequenceRequest::GetNextId(SEQ_MCP_SERVER_VALUE_ID.clone()))
                    .await
                {
                    id
                } else {
                    return handle_system_error(
                        "Unable to get next id from SequenceManager".to_string(),
                        "Failed to get next id from SequenceManager",
                    );
                };
                // 构建McpManagerRaftReq::PublishCurrentServer请求
                let raft_req = McpManagerRaftReq::PublishCurrentServer(server_id, server_value_id);
                let client_req = ClientRequest::McpReq { req: raft_req };

                // 通过RaftRequestRoute.request发送版本发布请求
                match appdata.raft_request_route.request(client_req).await {
                    Ok(response) => match response {
                        ClientResponse::Success => {
                            log::info!(
                                "Successfully published current version of McpServer: id={}, name={:?}, namespace={:?}, op_user={}",
                                server_id,
                                server.name,
                                server.namespace,
                                op_user
                            );
                            HttpResponse::Ok().json(ApiResult::success(Some(true)))
                        }
                        ClientResponse::McpResp { resp } => {
                            log::debug!("MCP Raft publish current server response: {:?}", resp);
                            log::info!(
                                "Successfully published current version of McpServer: id={}, name={:?}, namespace={:?}, op_user={}",
                                server_id,
                                server.name,
                                server.namespace,
                                op_user
                            );
                            HttpResponse::Ok().json(ApiResult::success(Some(true)))
                        }
                        _ => {
                            log::error!(
                                "Unexpected response when publishing current version of McpServer: id={}, op_user={}",
                                server_id,
                                op_user
                            );
                            handle_unexpected_response_error(
                                "Raft publish current McpServer version",
                            )
                        }
                    },
                    Err(err) => {
                        log::error!(
                            "Failed to publish current version of McpServer: id={}, op_user={}, error={}",
                            server_id,
                            op_user,
                            err
                        );
                        handle_raft_error(err, "publish current McpServer version")
                    }
                }
            }
            Ok(McpManagerResult::ServerInfo(None)) => {
                log::warn!(
                    "Attempted to publish current version of non-existent McpServer: id={}, op_user={}",
                    server_id,
                    op_user
                );
                handle_not_found_error("McpServer", &server_id.to_string())
            }
            Ok(_) => {
                log::error!(
                    "Unexpected response type when checking McpServer existence for publish: id={}, op_user={}",
                    server_id,
                    op_user
                );
                handle_unexpected_response_error("MCP Manager get McpServer for publish")
            }
            Err(err) => {
                log::error!(
                    "Failed to check McpServer existence before publish: id={}, op_user={}, error={}",
                    server_id,
                    op_user,
                    err
                );
                handle_mcp_manager_error(err, "check McpServer existence for publish")
            }
        },
        Err(err) => {
            log::error!(
                "Unable to connect to MCP Manager for publish check: id={}, op_user={}, error={}",
                server_id,
                op_user,
                err
            );
            handle_system_error(
                format!("Unable to connect to MCP Manager: {}", err),
                "Failed to send publish check request to MCP Manager",
            )
        }
    }
}

/// 发布历史版本
pub async fn publish_history_mcp_server(
    req: HttpRequest,
    appdata: web::Data<Arc<AppShareData>>,
    web::Json(param): web::Json<McpServerHistoryPublishParams>,
) -> impl Responder {
    // 验证参数
    if let Err(err) = param.validate() {
        return handle_param_error(err, "McpServer history publish parameter validation failed");
    }

    let server_id = param.id;
    let history_value_id = param.history_value_id;

    // 从HttpRequest中提取用户会话信息作为op_user
    let op_user = req
        .extensions()
        .get::<Arc<UserSession>>()
        .map(|session| session.username.clone())
        .unwrap_or_else(|| Arc::new("system".to_string()));

    log::debug!(
        "Publish history version of McpServer: id={}, history_value_id={}, op_user={:?}",
        server_id,
        history_value_id,
        op_user
    );

    // 首先检查McpServer是否存在
    let check_cmd = McpManagerReq::GetServer(server_id);
    match appdata.mcp_manager.send(check_cmd).await {
        Ok(res) => match res {
            Ok(McpManagerResult::ServerInfo(Some(server))) => {
                log::debug!(
                    "Found McpServer to publish history version: id={}, name={:?}, namespace={:?}",
                    server_id,
                    server.name,
                    server.namespace
                );

                // 记录版本回滚发布的操作日志，包含原版本、目标版本、发布时间和操作人信息
                log::info!(
                    "User {} is publishing history version of McpServer: id={}, name={:?}, namespace={:?}, target_history_value_id={}",
                    op_user,
                    server_id,
                    server.name,
                    server.namespace,
                    history_value_id
                );

                // 构建McpManagerRaftReq::PublishHistoryServer请求
                let raft_req = McpManagerRaftReq::PublishHistoryServer(server_id, history_value_id);
                let client_req = ClientRequest::McpReq { req: raft_req };

                // 通过RaftRequestRoute.request发送历史版本回滚发布请求
                match appdata.raft_request_route.request(client_req).await {
                    Ok(response) => match response {
                        ClientResponse::Success => {
                            log::info!(
                                "Successfully published history version of McpServer: id={}, name={:?}, namespace={:?}, history_value_id={}, op_user={}",
                                server_id,
                                server.name,
                                server.namespace,
                                history_value_id,
                                op_user
                            );
                            HttpResponse::Ok().json(ApiResult::success(Some(true)))
                        }
                        ClientResponse::McpResp { resp } => {
                            log::debug!("MCP Raft publish history server response: {:?}", resp);
                            log::info!(
                                "Successfully published history version of McpServer: id={}, name={:?}, namespace={:?}, history_value_id={}, op_user={}",
                                server_id,
                                server.name,
                                server.namespace,
                                history_value_id,
                                op_user
                            );
                            HttpResponse::Ok().json(ApiResult::success(Some(true)))
                        }
                        _ => {
                            log::error!(
                                "Unexpected response when publishing history version of McpServer: id={}, history_value_id={}, op_user={}",
                                server_id,
                                history_value_id,
                                op_user
                            );
                            handle_unexpected_response_error(
                                "Raft publish history McpServer version",
                            )
                        }
                    },
                    Err(err) => {
                        log::error!(
                            "Failed to publish history version of McpServer: id={}, history_value_id={}, op_user={}, error={}",
                            server_id,
                            history_value_id,
                            op_user,
                            err
                        );
                        handle_raft_error(err, "publish history McpServer version")
                    }
                }
            }
            Ok(McpManagerResult::ServerInfo(None)) => {
                log::warn!(
                    "Attempted to publish history version of non-existent McpServer: id={}, history_value_id={}, op_user={}",
                    server_id,
                    history_value_id,
                    op_user
                );
                handle_not_found_error("McpServer", &server_id.to_string())
            }
            Ok(_) => {
                log::error!(
                    "Unexpected response type when checking McpServer existence for history publish: id={}, history_value_id={}, op_user={}",
                    server_id,
                    history_value_id,
                    op_user
                );
                handle_unexpected_response_error("MCP Manager get McpServer for history publish")
            }
            Err(err) => {
                log::error!(
                    "Failed to check McpServer existence before history publish: id={}, history_value_id={}, op_user={}, error={}",
                    server_id,
                    history_value_id,
                    op_user,
                    err
                );
                handle_mcp_manager_error(err, "check McpServer existence for history publish")
            }
        },
        Err(err) => {
            log::error!(
                "Unable to connect to MCP Manager for history publish check: id={}, history_value_id={}, op_user={}, error={}",
                server_id,
                history_value_id,
                op_user,
                err
            );
            handle_system_error(
                format!("Unable to connect to MCP Manager: {}", err),
                "Failed to send history publish check request to MCP Manager",
            )
        }
    }
}
