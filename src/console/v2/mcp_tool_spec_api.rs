use crate::common::appdata::AppShareData;
use crate::common::constant::{EMPTY_ARC_STRING, SEQ_TOOL_SPEC_VERSION};
use crate::common::model::{ApiResult, PageResult, UserSession};
use crate::console::model::mcp_tool_spec_model::{
    ToolSpecImportDto, ToolSpecParams, ToolSpecQueryRequest,
};
use crate::console::v2::{
    handle_mcp_manager_error, handle_not_found_error, handle_param_error, handle_raft_error,
    handle_system_error, handle_unexpected_response_error,
};
use crate::mcp::model::actor_model::{
    McpManagerRaftReq, McpManagerReq, McpManagerResult, ToolSpecDto,
};
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

/// 批量导出ToolSpec
pub async fn download_tool_specs(
    _req: HttpRequest,
    request: web::Query<ToolSpecQueryRequest>,
    appdata: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    // 验证查询参数
    if let Err(err) = request.validate() {
        return handle_param_error(err, "ToolSpec download parameter validation failed");
    }

    // 转换查询参数，设置最大限制10万
    let mut query_param = request.to_query_param();
    query_param.limit = 100_000;
    query_param.offset = 0;

    // 发送查询请求到MCP Manager
    let cmd = McpManagerReq::QueryToolSpec(query_param);
    match appdata.mcp_manager.send(cmd).await {
        Ok(res) => match res {
            Ok(McpManagerResult::ToolSpecPageInfo(_, list)) => {
                let mut tmpfile: File = tempfile::tempfile().unwrap();
                {
                    let write = std::io::Write::by_ref(&mut tmpfile);
                    let zip = ZipWriter::new(write);
                    generate_tool_spec_zip(zip, list).ok();
                }
                // Seek to start
                tmpfile.seek(SeekFrom::Start(0)).unwrap();
                let mut buf = vec![];
                tmpfile.read_to_end(&mut buf).unwrap();

                let filename = format!("rnacos_toolspec_export_{}.zip", crate::now_millis());
                HttpResponse::Ok()
                    .insert_header(actix_web::http::header::ContentType::octet_stream())
                    .insert_header(actix_web::http::header::ContentDisposition::attachment(
                        filename,
                    ))
                    .body(buf)
            }
            Ok(_) => handle_unexpected_response_error("MCP Manager download ToolSpec"),
            Err(err) => handle_mcp_manager_error(err, "download ToolSpec"),
        },
        Err(err) => handle_system_error(
            format!("Unable to connect to MCP Manager: {}", err),
            "Failed to send download request to MCP Manager",
        ),
    }
}

/// 生成ToolSpec的zip文件
fn generate_tool_spec_zip(
    mut zip: ZipWriter<&mut File>,
    list: Vec<ToolSpecDto>,
) -> anyhow::Result<()> {
    if list.is_empty() {
        let options = FileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .unix_permissions(0o755);
        zip.start_file(".ignore", options)?;
        zip.write_all("empty toolspec".as_bytes())?;
    }

    for item in &list {
        // 生成YAML内容
        let yaml_content = generate_tool_spec_yaml(item);

        // 文件名格式: {group}_{tool_name}.yaml
        let filename = format!("{}_{}.yaml", item.group.as_str(), item.tool_name.as_str());

        let options = FileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .unix_permissions(0o755);

        zip.start_file(filename, options)?;
        zip.write_all(yaml_content.as_bytes())?;
    }

    zip.finish()?;
    Ok(())
}

/// 生成ToolSpec的YAML内容
fn generate_tool_spec_yaml(tool_spec: &ToolSpecDto) -> String {
    use crate::console::model::mcp_tool_spec_model::ToolSpecImportDto;

    // 转换为ToolSpecImportDto
    let import_dto = ToolSpecImportDto::from(tool_spec);

    // 使用serde_yml序列化
    serde_yml::to_string(&import_dto).unwrap_or_else(|_| {
        // 如果序列化失败，返回基本错误信息
        format!(
            "Failed to serialize ToolSpec: {}_{}",
            tool_spec.group.as_str(),
            tool_spec.tool_name.as_str()
        )
    })
}

#[derive(Debug, MultipartForm)]
pub struct ToolSpecUploadForm {
    #[multipart(rename = "namespace")]
    pub namespace: Option<Text<String>>,
    #[multipart(rename = "file")]
    pub files: Vec<TempFile>,
}

/// 批量导入ToolSpec
pub async fn import_tool_specs(
    req: HttpRequest,
    MultipartForm(form): MultipartForm<ToolSpecUploadForm>,
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

    let mut tool_spec_params = Vec::new();

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

                        // 解析YAML内容为ToolSpecImportDto
                        let import_dto: ToolSpecImportDto = match serde_yml::from_str(&content) {
                            Ok(dto) => dto,
                            Err(e) => {
                                log::warn!("Failed to parse YAML file {}: {}", filename_str, e);
                                continue;
                            }
                        };

                        // 转换为ToolSpecParams
                        let tool_spec_param = ToolSpecParams {
                            namespace: namespace.clone(),
                            group: import_dto.group.clone(),
                            tool_name: import_dto.name.clone(),
                            function: Some(crate::mcp::model::tools::ToolFunctionValue {
                                name: import_dto.name.clone(),
                                description: import_dto.description.clone(),
                                input_schema: import_dto.input_schema,
                            }),
                            op_user: op_user.clone(),
                        };

                        tool_spec_params.push(tool_spec_param);
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to open zip archive: {}", e);
                return Ok(HttpResponse::BadRequest().body("Invalid zip file format"));
            }
        }
    }

    // 如果没有有效的工具规范，返回错误
    if tool_spec_params.is_empty() {
        return Ok(
            HttpResponse::BadRequest().body("No valid tool specifications found in the zip file")
        );
    }

    // 一次性获取所需数量的版本号范围
    let mut seq_range = if let Ok(Ok(SequenceResult::Range(range))) = appdata
        .sequence_manager
        .send(SequenceRequest::GetDirectRange(
            SEQ_TOOL_SPEC_VERSION.clone(),
            tool_spec_params.len() as u64,
        ))
        .await
    {
        range
    } else {
        return Ok(HttpResponse::InternalServerError()
            .body("Failed to get version IDs from SequenceManager"));
    };

    // 转换为ToolSpecParam列表，为每个参数分配版本号
    let mut tool_spec_params_with_version = Vec::new();
    for param in tool_spec_params.iter() {
        let version = if let Some(id) = seq_range.next_id() {
            id
        } else {
            return Ok(
                HttpResponse::InternalServerError().body("Insufficient version IDs in range")
            );
        };

        let mut tool_spec_param = param.to_tool_spec_param(op_user.clone());
        tool_spec_param.version = version;
        tool_spec_params_with_version.push(tool_spec_param);
    }

    // 构建McpManagerRaftReq::UpdateToolSpecList请求
    let raft_req = McpManagerRaftReq::UpdateToolSpecList(tool_spec_params_with_version);
    let client_req = ClientRequest::McpReq { req: raft_req };

    // 通过RaftRequestRoute.request发送写入请求
    match appdata.raft_request_route.request(client_req).await {
        Ok(response) => match response {
            ClientResponse::Success => Ok(HttpResponse::Ok().json(ApiResult::success(Some(true)))),
            ClientResponse::McpResp { resp: _ } => {
                Ok(HttpResponse::Ok().json(ApiResult::success(Some(true))))
            }
            _ => {
                log::error!("Unexpected response from Raft batch import ToolSpec");
                Ok(
                    HttpResponse::InternalServerError().json(ApiResult::<bool>::error(
                        "RAFT_ERROR".to_string(),
                        Some("Unexpected response from Raft".to_string()),
                    )),
                )
            }
        },
        Err(err) => {
            log::error!("Raft batch import ToolSpec error: {}", err);
            Ok(
                HttpResponse::InternalServerError().json(ApiResult::<bool>::error(
                    "RAFT_ERROR".to_string(),
                    Some(format!("Failed to batch import ToolSpec: {}", err)),
                )),
            )
        }
    }
}
