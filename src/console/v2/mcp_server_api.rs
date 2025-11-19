use crate::common::appdata::AppShareData;
use crate::common::constant::{EMPTY_ARC_STRING, SEQ_MCP_SERVER_ID, SEQ_MCP_SERVER_VALUE_ID};
use crate::common::model::{ApiResult, PageResult, UserSession};
use crate::common::string_utils::StringUtils;
use crate::console::model::mcp_server_model::{
    McpServerHistoryPublishParams, McpServerHistoryQueryRequest, McpServerParams,
    McpServerQueryRequest, McpServerValueDto, McpSimpleToolParams,
};
use crate::console::v2::{
    handle_error, handle_mcp_manager_error, handle_not_found_error, handle_param_error,
    handle_system_error, handle_unexpected_response_error,
};
use crate::mcp::model::actor_model::{McpManagerRaftReq, McpManagerReq, McpManagerResult};
use crate::mcp::model::mcp::McpServerDto;
use crate::raft::store::{ClientRequest, ClientResponse};
use crate::sequence::{SequenceRequest, SequenceResult};
use actix_multipart::form::tempfile::TempFile;
use actix_multipart::form::text::Text;
use actix_multipart::form::MultipartForm;
use actix_web::{web, Error, HttpMessage, HttpRequest, HttpResponse, Responder};
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::sync::Arc;
use zip::write::FileOptions;
use zip::ZipWriter;

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

/// 批量导出McpServer
pub async fn download_mcp_servers(
    _req: HttpRequest,
    request: web::Query<McpServerQueryRequest>,
    appdata: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    // 验证查询参数
    if let Err(err) = request.validate() {
        return handle_param_error(err, "McpServer download parameter validation failed");
    }

    // 转换查询参数，设置最大限制10万
    let mut query_param = request.to_mcp_query_param();
    query_param.limit = 100_000;
    query_param.offset = 0;

    // 发送查询请求到MCP Manager
    let cmd = McpManagerReq::QueryServer(query_param);
    match appdata.mcp_manager.send(cmd).await {
        Ok(res) => match res {
            Ok(McpManagerResult::ServerPageInfo(_, list)) => {
                let mut tmpfile: File = tempfile::tempfile().unwrap();
                {
                    let write = std::io::Write::by_ref(&mut tmpfile);
                    let zip = ZipWriter::new(write);
                    generate_mcp_server_zip(zip, list).ok();
                }
                // Seek to start
                tmpfile.seek(SeekFrom::Start(0)).unwrap();
                let mut buf = vec![];
                tmpfile.read_to_end(&mut buf).unwrap();

                let filename = format!("rnacos_mcserver_export_{}.zip", crate::now_millis());
                HttpResponse::Ok()
                    .insert_header(actix_web::http::header::ContentType::octet_stream())
                    .insert_header(actix_web::http::header::ContentDisposition::attachment(
                        filename,
                    ))
                    .body(buf)
            }
            Ok(_) => handle_unexpected_response_error("MCP Manager download McpServer"),
            Err(err) => handle_mcp_manager_error(err, "download McpServer"),
        },
        Err(err) => handle_system_error(
            format!("Unable to connect to MCP Manager: {}", err),
            "Failed to send download request to MCP Manager",
        ),
    }
}

/// 生成McpServer的zip文件
fn generate_mcp_server_zip(
    mut zip: ZipWriter<&mut File>,
    list: Vec<McpServerDto>,
) -> anyhow::Result<()> {
    if list.is_empty() {
        let options = FileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .unix_permissions(0o755);
        zip.start_file(".ignore", options)?;
        zip.write_all("empty mcpservers".as_bytes())?;
    }

    for item in &list {
        // 生成YAML内容
        let yaml_content = generate_mcp_server_yaml(item);

        // 文件名格式: {unique_key}.yaml
        let filename = format!("{}.yaml", item.unique_key.as_str());

        let options = FileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .unix_permissions(0o755);

        zip.start_file(filename, options)?;
        zip.write_all(yaml_content.as_bytes())?;
    }

    zip.finish()?;
    Ok(())
}

/// 生成McpServer的YAML内容
fn generate_mcp_server_yaml(mcp_server: &McpServerDto) -> String {
    use crate::console::model::mcp_server_model::McpServerImportDto;

    // 转换为McpServerImportDto
    let import_dto = McpServerImportDto::from(mcp_server);

    // 使用serde_yml序列化
    serde_yml::to_string(&import_dto).unwrap_or_else(|_| {
        // 如果序列化失败，返回基本错误信息
        format!(
            "Failed to serialize McpServer: {}",
            mcp_server.unique_key.as_str()
        )
    })
}

#[derive(Debug, MultipartForm)]
pub struct McpServerUploadForm {
    #[multipart(rename = "namespace")]
    pub namespace: Option<Text<String>>,
    #[multipart(rename = "file")]
    pub files: Vec<TempFile>,
}

/// 批量导入McpServer
pub async fn import_mcp_servers(
    req: HttpRequest,
    MultipartForm(form): MultipartForm<McpServerUploadForm>,
    appdata: web::Data<Arc<AppShareData>>,
) -> Result<impl Responder, Error> {
    // 获取命名空间，优先使用表单中的namespace，然后使用header中的namespace,最后使用默认值
    let mut namespace = if let Some(namespace_text) = form.namespace {
        Arc::new(namespace_text.into_inner())
    } else {
        match req.headers().get("namespace") {
            Some(v) => Arc::new(String::from_utf8_lossy(v.as_bytes()).to_string()),
            None => EMPTY_ARC_STRING.clone(),
        }
    };
    if namespace.is_empty() {
        namespace = Arc::new(crate::namespace::default_namespace("".to_string()));
    }

    // 检查命名空间权限
    let namespace_privilege = crate::user_namespace_privilege!(req);
    if !namespace_privilege.check_permission(&namespace) {
        return Ok(HttpResponse::Unauthorized().body(format!(
            "user no such namespace permission: {}",
            namespace.as_str()
        )));
    }

    // 从HttpRequest中提取用户会话信息作为op_user
    let op_user = req
        .extensions()
        .get::<Arc<UserSession>>()
        .map(|session| session.username.clone());

    let mut mcp_server_params = Vec::new();

    // 处理上传的文件
    for f in form.files {
        match zip::ZipArchive::new(f.file) {
            Ok(mut archive) => {
                for i in 0..archive.len() {
                    let mut file = archive.by_index(i).unwrap();
                    let filename = file.name();

                    // 只处理.yaml文件，跳过目录
                    let filename_str = filename.to_string();
                    if !filename_str.ends_with('/') && filename_str.ends_with(".yaml") {
                        // 读取文件内容
                        let content = match io::read_to_string(&mut file) {
                            Ok(v) => v,
                            Err(e) => {
                                log::warn!("Failed to read file {}: {}", filename_str, e);
                                continue;
                            }
                        };

                        // 解析YAML内容为McpServerImportDto
                        use crate::console::model::mcp_server_model::McpServerImportDto;
                        let import_dto: McpServerImportDto = match serde_yml::from_str(&content) {
                            Ok(dto) => dto,
                            Err(e) => {
                                log::warn!("Failed to parse YAML file {}: {}", filename_str, e);
                                continue;
                            }
                        };

                        // 转换为McpServerParams
                        let mcp_server_param = McpServerParams {
                            id: None, // 导入时不设置ID，由系统生成
                            unique_key: Some(import_dto.unique_key.clone()),
                            namespace: Some(namespace.as_str().to_string()),
                            name: Some(import_dto.name.clone()),
                            description: Some(import_dto.description.clone()),
                            auth_keys: Some(import_dto.auth_keys.clone()),
                            tools: Some(
                                import_dto
                                    .tools
                                    .iter()
                                    .map(|tool| McpSimpleToolParams {
                                        id: None,
                                        tool_name: Arc::new(tool.tool_name.clone()),
                                        namespace: namespace.clone(),
                                        group: Arc::new(tool.tool_group.clone()),
                                        tool_version: None,
                                        route_rule: Some(tool.route_rule.clone()),
                                    })
                                    .collect(),
                            ),
                        };

                        mcp_server_params.push(mcp_server_param);
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to open zip archive: {}", e);
                return Ok(HttpResponse::BadRequest().body("Invalid zip file format"));
            }
        }
    }

    // 如果没有有效的MCP服务器，返回错误
    if mcp_server_params.is_empty() {
        return Ok(HttpResponse::BadRequest().body("No valid MCP servers found in the zip file"));
    }

    // 批量处理每个McpServer
    let mut imported_count = 0;
    let mut updated_count = 0;
    let mut errors = Vec::new();

    for param in mcp_server_params {
        match update_mcp_server_for_import(param.clone(), op_user.clone(), &appdata).await {
            Ok(is_updated) => {
                if is_updated {
                    updated_count += 1;
                } else {
                    imported_count += 1;
                }
            }
            Err(e) => {
                let server_key = param.unique_key.as_deref().unwrap_or("unknown");
                errors.push(format!("Failed to import server {}: {}", server_key, e));
            }
        }
    }

    // 构建响应
    if errors.is_empty() {
        let result_msg = format!(
            "{} servers created, {} servers updated",
            imported_count, updated_count
        );
        Ok(HttpResponse::Ok().json(ApiResult::success(Some(result_msg))))
    } else {
        let error_summary = if errors.len() > 10 {
            format!(
                "{} servers created, {} servers updated, {} errors. First errors: {:?}",
                imported_count,
                updated_count,
                errors.len(),
                &errors[..10]
            )
        } else {
            format!(
                "{} servers created, {} servers updated, {} errors: {:?}",
                imported_count,
                updated_count,
                errors.len(),
                errors
            )
        };
        Ok(HttpResponse::Ok().json(ApiResult::<String>::error(
            "PARTIAL_SUCCESS".to_string(),
            Some(error_summary),
        )))
    }
}

/// 为导入操作添加或更新McpServer的内部函数
async fn update_mcp_server_for_import(
    param: McpServerParams,
    op_user: Option<Arc<String>>,
    appdata: &web::Data<Arc<AppShareData>>,
) -> anyhow::Result<bool> {
    // 验证参数
    param.validate()?;

    // 检查是否已存在相同unique_key的服务
    if let Some(server_key) = param.unique_key.as_ref() {
        if !server_key.is_empty() {
            let check_result = appdata
                .mcp_manager
                .send(McpManagerReq::GetServerByKey(Arc::new(
                    server_key.to_string(),
                )))
                .await??;

            if let McpManagerResult::ServerInfo(Some(existing_server)) = check_result {
                // 服务已存在，执行更新操作
                log::info!(
                    "Updating existing McpServer with key: {} (id: {})",
                    server_key,
                    existing_server.id
                );

                // 设置ID为现有服务的ID
                let mut update_param = param.clone();
                update_param.id = Some(existing_server.id);

                // 转换为McpServerParam用于更新
                let mut server_param = update_param.to_mcp_server_param(op_user.clone());
                // 保留原有的unique_key，确保不会改变
                server_param.unique_key = Some(Arc::new(server_key.clone()));

                // 构建McpManagerRaftReq::UpdateServer请求
                let raft_req = McpManagerRaftReq::UpdateServer(server_param);
                let client_req = ClientRequest::McpReq { req: raft_req };

                // 通过RaftRequestRoute.request发送更新请求
                let response = appdata.raft_request_route.request(client_req).await?;

                match response {
                    ClientResponse::McpResp { resp: _ } => return Ok(true), // 返回true表示更新
                    _ => {
                        return Err(anyhow::anyhow!(
                            "Unexpected response from Raft update McpServer"
                        ))
                    }
                }
            }
        }
    }

    // 服务不存在，执行创建操作
    log::info!("Creating new McpServer");

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
        ClientResponse::McpResp { resp: _ } => Ok(false), // 返回false表示创建
        _ => Err(anyhow::anyhow!(
            "Unexpected response from Raft add McpServer"
        )),
    }
}
