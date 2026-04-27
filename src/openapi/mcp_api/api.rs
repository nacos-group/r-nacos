use crate::common::appdata::AppShareData;
use crate::common::model::{ApiResult, PageResult};
use crate::mcp::model::actor_model::{McpManagerReq, McpManagerResult};
use crate::openapi::mcp_api::model::{
    OpenMcpServerDto, OpenMcpServerQueryParam, OpenToolFunctionValue, OpenToolSpecQueryParam,
};
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use std::sync::Arc;

pub async fn query_mcp_server_list(
    _req: HttpRequest,
    query: web::Query<OpenMcpServerQueryParam>,
    appdata: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    let param = query.into_inner();

    if let Err(err) = param.validate() {
        return HttpResponse::BadRequest().json(ApiResult::<String>::error(
            "INVALID_PARAM".to_string(),
            Some(format!("Parameter validation failed: {}", err)),
        ));
    }

    let mcp_query_param = param.to_mcp_query_param();
    let cmd = McpManagerReq::QueryServer(mcp_query_param);

    match appdata.mcp_manager.send(cmd).await {
        Ok(Ok(McpManagerResult::ServerPageInfo(total_count, list))) => {
            let open_dto_list: Vec<OpenMcpServerDto> =
                list.iter().map(OpenMcpServerDto::from_server_dto).collect();
            HttpResponse::Ok().json(ApiResult::success(Some(PageResult {
                total_count,
                list: open_dto_list,
            })))
        }
        Ok(Ok(_)) => HttpResponse::InternalServerError().json(ApiResult::<String>::error(
            "UNEXPECTED_RESPONSE".to_string(),
            Some("Unexpected response from MCP Manager".to_string()),
        )),
        Ok(Err(err)) => HttpResponse::InternalServerError().json(ApiResult::<String>::error(
            "MCP_MANAGER_ERROR".to_string(),
            Some(format!("MCP Manager error: {}", err)),
        )),
        Err(err) => HttpResponse::InternalServerError().json(ApiResult::<String>::error(
            "SYSTEM_ERROR".to_string(),
            Some(format!("Unable to connect to MCP Manager: {}", err)),
        )),
    }
}

pub async fn query_tool_spec_list(
    _req: HttpRequest,
    query: web::Query<OpenToolSpecQueryParam>,
    appdata: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    let param = query.into_inner();

    if let Err(err) = param.validate() {
        return HttpResponse::BadRequest().json(ApiResult::<String>::error(
            "INVALID_PARAM".to_string(),
            Some(format!("Parameter validation failed: {}", err)),
        ));
    }

    let tool_spec_query_param = param.to_tool_spec_query_param();
    let cmd = McpManagerReq::QueryToolSpec(tool_spec_query_param);

    match appdata.mcp_manager.send(cmd).await {
        Ok(Ok(McpManagerResult::ToolSpecPageInfo(total_count, list))) => {
            let tool_list: Vec<OpenToolFunctionValue> = list
                .iter()
                .map(|dto| {
                    OpenToolFunctionValue::new(
                        dto.namespace.clone(),
                        dto.group.clone(),
                        dto.function.as_ref().clone(),
                    )
                })
                .collect();
            HttpResponse::Ok().json(ApiResult::success(Some(PageResult {
                total_count,
                list: tool_list,
            })))
        }
        Ok(Ok(_)) => HttpResponse::InternalServerError().json(ApiResult::<String>::error(
            "UNEXPECTED_RESPONSE".to_string(),
            Some("Unexpected response from MCP Manager".to_string()),
        )),
        Ok(Err(err)) => HttpResponse::InternalServerError().json(ApiResult::<String>::error(
            "MCP_MANAGER_ERROR".to_string(),
            Some(format!("MCP Manager error: {}", err)),
        )),
        Err(err) => HttpResponse::InternalServerError().json(ApiResult::<String>::error(
            "SYSTEM_ERROR".to_string(),
            Some(format!("Unable to connect to MCP Manager: {}", err)),
        )),
    }
}
