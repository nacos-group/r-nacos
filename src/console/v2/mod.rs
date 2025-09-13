use crate::common::model::ApiResult;
use actix_web::HttpResponse;

pub mod cluster_api;
pub mod config_api;
pub mod login_api;
pub mod mcp_server_api;
pub mod mcp_tool_spec_api;
pub mod metrics_api;
pub mod namespace_api;
pub mod naming_api;
pub mod user_api;

pub const ERROR_CODE_SYSTEM_ERROR: &str = "SYSTEM_ERROR";
pub const ERROR_CODE_PARAM_ERROR: &str = "PARAM_ERROR";
pub const ERROR_CODE_NOT_FOUND: &str = "NOT_FOUND";
pub const ERROR_CODE_MCP_MANAGER_ERROR: &str = "MCP_MANAGER_ERROR";
pub const ERROR_CODE_RAFT_ERROR: &str = "RAFT_ERROR";

pub enum ApiResponse<T>
where
    T: Sized + Default + serde::Serialize,
{
    Result(ApiResult<T>),
    Response(HttpResponse),
}

impl<T> From<ApiResponse<T>> for HttpResponse
where
    T: Sized + Default + serde::Serialize,
{
    fn from(value: ApiResponse<T>) -> Self {
        match value {
            ApiResponse::Result(v) => HttpResponse::Ok().json(v),
            ApiResponse::Response(v) => v,
        }
    }
}

/// 统一的错误处理函数
pub fn handle_system_error(error: impl std::fmt::Display, context: &str) -> HttpResponse {
    log::error!("{}: {}", context, error);
    HttpResponse::Ok().json(ApiResult::<()>::error(
        ERROR_CODE_SYSTEM_ERROR.to_string(),
        Some(error.to_string()),
    ))
}

/// 处理参数验证错误
pub fn handle_param_error(error: impl std::fmt::Display, context: &str) -> HttpResponse {
    log::warn!("{}: {}", context, error);
    HttpResponse::Ok().json(ApiResult::<()>::error(
        ERROR_CODE_PARAM_ERROR.to_string(),
        Some(error.to_string()),
    ))
}

/// 处理资源不存在错误
pub fn handle_not_found_error(resource: &str, identifier: &str) -> HttpResponse {
    log::debug!("{} not found: {}", resource, identifier);
    HttpResponse::Ok().json(ApiResult::<()>::error(
        ERROR_CODE_NOT_FOUND.to_string(),
        Some(format!("{} not found", resource)),
    ))
}

/// 处理MCP Manager错误
pub fn handle_mcp_manager_error(error: impl std::fmt::Display, operation: &str) -> HttpResponse {
    log::error!("MCP Manager {} operation failed: {}", operation, error);
    HttpResponse::Ok().json(ApiResult::<()>::error(
        ERROR_CODE_MCP_MANAGER_ERROR.to_string(),
        Some(format!(
            "MCP Manager {} operation failed: {}",
            operation, error
        )),
    ))
}

/// 处理Raft错误
pub fn handle_raft_error(error: impl std::fmt::Display, operation: &str) -> HttpResponse {
    log::error!("Raft {} operation failed: {}", operation, error);
    HttpResponse::Ok().json(ApiResult::<()>::error(
        ERROR_CODE_RAFT_ERROR.to_string(),
        Some(format!("Raft {} operation failed: {}", operation, error)),
    ))
}

/// 处理意外响应类型错误
pub fn handle_unexpected_response_error(context: &str) -> HttpResponse {
    log::error!("{} returned unexpected response type", context);
    HttpResponse::Ok().json(ApiResult::<()>::error(
        ERROR_CODE_SYSTEM_ERROR.to_string(),
        Some("Unexpected response type".to_string()),
    ))
}

/// 处理 anyhow::Result 错误
pub fn handle_error(error: anyhow::Error) -> HttpResponse {
    log::error!("Operation failed: {}", error);
    HttpResponse::Ok().json(ApiResult::<()>::error(
        ERROR_CODE_SYSTEM_ERROR.to_string(),
        Some(error.to_string()),
    ))
}
