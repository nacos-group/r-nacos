#![allow(unused_imports)]

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::sync::Arc;

use actix_multipart::form::tempfile::TempFile;
use actix_multipart::form::text::Text;
use actix_multipart::form::MultipartForm;
use actix_web::{http::header, web, Error, HttpMessage, HttpRequest, HttpResponse, Responder};
use zip::write::FileOptions;

use super::model::PageResult;
use crate::common::appdata::AppShareData;
use crate::config::core::{ConfigActor, ConfigCmd, ConfigInfoDto, ConfigKey, ConfigResult};
use crate::config::config_type::ConfigType;
use crate::config::ConfigUtils;
use crate::console::model::config_model::{
    ConfigParams, OpsConfigOptQueryListResponse, OpsConfigQueryListRequest,
};
use crate::raft::cluster::model::SetConfigReq;
use crate::{now_millis, user_namespace_privilege};
use actix::prelude::Addr;
use tokio_stream::StreamExt;
use zip::ZipWriter;

pub async fn query_config_list(
    req: HttpRequest,
    request: web::Query<OpsConfigQueryListRequest>,
    config_addr: web::Data<Addr<ConfigActor>>,
) -> impl Responder {
    let param = request.0.to_param(&req).unwrap();
    if !param
        .namespace_privilege
        .check_option_value_permission(&param.tenant, true)
    {
        return HttpResponse::Unauthorized().body(format!(
            "user no such namespace permission: {:?}",
            &param.tenant
        ));
    }
    let cmd = ConfigCmd::QueryPageInfo(Box::new(param));
    match config_addr.send(cmd).await {
        Ok(res) => {
            let r: ConfigResult = res.unwrap();
            match r {
                ConfigResult::ConfigInfoPage(size, list) => {
                    let response = OpsConfigOptQueryListResponse {
                        count: size as u64,
                        list,
                    };
                    let v = serde_json::to_string(&response).unwrap();
                    HttpResponse::Ok()
                        .insert_header(header::ContentType(mime::APPLICATION_JSON))
                        .body(v)
                }
                _ => HttpResponse::InternalServerError().body("config result error"),
            }
        }
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

pub async fn query_history_config_page(
    request: web::Query<OpsConfigQueryListRequest>,
    config_addr: web::Data<Addr<ConfigActor>>,
) -> impl Responder {
    let param = match request.0.to_history_param() {
        Ok(param) => param,
        Err(err) => {
            return HttpResponse::InternalServerError().body(err.to_string());
        }
    };
    let cmd = ConfigCmd::QueryHistoryPageInfo(Box::new(param));
    match config_addr.send(cmd).await {
        Ok(res) => {
            let r: ConfigResult = res.unwrap();
            match r {
                ConfigResult::ConfigHistoryInfoPage(size, list) => {
                    let response = PageResult {
                        count: size as u64,
                        list,
                    };
                    let v = serde_json::to_string(&response).unwrap();
                    HttpResponse::Ok()
                        .insert_header(header::ContentType(mime::APPLICATION_JSON))
                        .body(v)
                }
                _ => HttpResponse::InternalServerError().body("config result error"),
            }
        }
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

#[derive(Debug, MultipartForm)]
pub struct UploadForm {
    #[multipart(rename = "tenant")]
    pub tenant: Option<Text<String>>,
    #[multipart(rename = "file")]
    pub files: Vec<TempFile>,
}

pub async fn import_config(
    req: HttpRequest,
    MultipartForm(form): MultipartForm<UploadForm>,
    app: web::Data<Arc<AppShareData>>,
) -> Result<impl Responder, Error> {
    let tenant = Arc::new(ConfigUtils::default_tenant(
        match req.headers().get("tenant") {
            Some(v) => String::from_utf8_lossy(v.as_bytes()).to_string(),
            None => "".to_owned(),
        },
    ));
    let namespace_privilege = user_namespace_privilege!(req);
    if !namespace_privilege.check_permission(&tenant) {
        return Ok(HttpResponse::Unauthorized().body(format!(
            "user no such namespace permission: {}",
            tenant.as_str()
        )));
    }
    //let tenant = Arc::new(ConfigUtils::default_tenant(config_info.0.tenant.unwrap_or_default()));
    for f in form.files {
        match zip::ZipArchive::new(f.file) {
            Ok(mut archive) => {
                // 第一遍：收集 .meta 文件中的 config type 信息
                let mut meta_types: std::collections::HashMap<String, String> =
                    std::collections::HashMap::new();
                for i in 0..archive.len() {
                    let mut file = archive.by_index(i).unwrap();
                    let filename = file.name().to_owned();
                    // nacos .meta 文件格式: group/.dataId.meta
                    if filename.ends_with(".meta") {
                        if let Ok(meta_str) = io::read_to_string(&mut file) {
                            // 解析 .meta JSON: {"type":"yaml","appName":"","desc":""}
                            if let Ok(meta_json) =
                                serde_json::from_str::<serde_json::Value>(&meta_str)
                            {
                                if let Some(config_type) =
                                    meta_json.get("type").and_then(|v| v.as_str())
                                {
                                    // 从 "group/.dataId.meta" 提取 "group/dataId"
                                    let parts: Vec<&str> = filename.split('/').collect();
                                    if parts.len() == 2 {
                                        let meta_name = parts[1];
                                        if let Some(data_id) = meta_name
                                            .strip_prefix('.')
                                            .and_then(|s| s.strip_suffix(".meta"))
                                        {
                                            let key =
                                                format!("{}/{}", parts[0], data_id);
                                            meta_types.insert(
                                                key,
                                                config_type.to_owned(),
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // 第二遍：导入配置文件
                for i in 0..archive.len() {
                    let mut file = archive.by_index(i).unwrap();
                    let filename = file.name().to_owned();
                    // 跳过目录和 .meta 文件
                    if filename.ends_with('/') || filename.ends_with(".meta") {
                        continue;
                    }
                    let parts: Vec<&str> = filename.split('/').collect();
                    if parts.len() != 2 {
                        continue;
                    }
                    let config_key = ConfigKey::new_by_arc(
                        Arc::new(parts[1].to_owned()),
                        Arc::new(parts[0].to_owned()),
                        tenant.clone(),
                    );
                    let value = match io::read_to_string(&mut file) {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    let mut req = SetConfigReq::new(config_key.clone(), Arc::new(value));

                    // 优先从 .meta 文件获取 config type，回退到文件扩展名检测
                    let meta_key = format!("{}/{}", parts[0], parts[1]);
                    if let Some(ct) = meta_types.get(&meta_key) {
                        req.config_type =
                            Some(ConfigType::new_by_value(ct).get_value());
                    } else {
                        let data_id_clone = config_key.data_id.clone();
                        req.config_type =
                            SetConfigReq::detect_config_type(data_id_clone);
                    }

                    app.config_route.set_config(req).await.ok();
                }
            }
            Err(_) => todo!(),
        }
    }
    Ok(HttpResponse::Ok().finish())
}

fn zip_file(mut zip: ZipWriter<&mut File>, list: Vec<ConfigInfoDto>) -> anyhow::Result<()> {
    if list.is_empty() {
        let options = FileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .unix_permissions(0o755);
        zip.start_file(".ignore", options)?;
        zip.write_all("empty config".as_bytes())?;
    }
    for item in &list {
        zip.add_directory(item.group.as_str(), Default::default())
            .ok();
        let options = FileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .unix_permissions(0o755);
        zip.start_file(
            format!("{}/{}", &item.group.as_str(), &item.data_id.as_str()),
            options,
        )?;
        zip.write_all(item.content.as_ref().unwrap().as_bytes())?;

        // 写入 .meta 文件，兼容 nacos 导出格式
        if let Some(config_type) = &item.config_type {
            if config_type.as_str() != "text" {
                let meta_options = FileOptions::default()
                    .compression_method(zip::CompressionMethod::Stored)
                    .unix_permissions(0o755);
                zip.start_file(
                    format!("{}/.{}.meta", &item.group.as_str(), &item.data_id.as_str()),
                    meta_options,
                )?;
                let desc = item.desc.as_ref().map(|d| d.as_str()).unwrap_or("");
                let meta_content = format!(
                    r#"{{"type":"{}","appName":"","desc":"{}"}}"#,
                    config_type.as_str(),
                    desc
                );
                zip.write_all(meta_content.as_bytes())?;
            }
        }
    }
    zip.finish()?;
    Ok(())
}

///
/// 按查询条件导出配置
pub async fn download_config(
    req: HttpRequest,
    request: web::Query<OpsConfigQueryListRequest>,
    config_addr: web::Data<Addr<ConfigActor>>,
) -> impl Responder {
    let mut param = request.0.to_param(&req).unwrap();
    param.limit = 0xffff_ffff;
    param.query_context = true;
    let cmd = ConfigCmd::QueryPageInfo(Box::new(param));
    match config_addr.send(cmd).await {
        Ok(res) => {
            let r: ConfigResult = res.unwrap();
            match r {
                ConfigResult::ConfigInfoPage(_, list) => {
                    let mut tmpfile: File = tempfile::tempfile().unwrap();
                    {
                        let write = std::io::Write::by_ref(&mut tmpfile);
                        let zip = ZipWriter::new(write);
                        zip_file(zip, list).ok();
                    }
                    // Seek to start
                    tmpfile.seek(SeekFrom::Start(0)).unwrap();
                    let mut buf = vec![];
                    tmpfile.read_to_end(&mut buf).unwrap();

                    let filename = format!("rnacos_config_export_{}.zip", now_millis());
                    HttpResponse::Ok()
                        .insert_header(header::ContentType::octet_stream())
                        .insert_header(header::ContentDisposition::attachment(filename))
                        .body(buf)
                }
                _ => HttpResponse::InternalServerError().body("config result error"),
            }
        }
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

/// 按 key 导出配置
pub async fn download_config_by_keys(
    request: web::Json<Vec<ConfigParams>>,
    config_addr: web::Data<Addr<ConfigActor>>,
) -> impl Responder {
    let params = request.into_inner();
    if params.is_empty() {
        return HttpResponse::BadRequest().body("keys cannot be empty");
    }

    let keys = params
        .into_iter()
        .map(|k| {
            let k = k.to_key();
            ConfigKey {
                tenant: Arc::new(ConfigUtils::default_tenant(k.tenant.to_string())),
                ..k
            }
        })
        .collect();

    let cmd = ConfigCmd::QueryInfoByKeys(Box::new(keys));
    match config_addr.send(cmd).await {
        Ok(res) => {
            let r: ConfigResult = res.unwrap();
            match r {
                ConfigResult::ConfigInfoPage(_, list) => {
                    let mut tmpfile: File = tempfile::tempfile().unwrap();
                    {
                        let write = std::io::Write::by_ref(&mut tmpfile);
                        let zip = ZipWriter::new(write);
                        zip_file(zip, list).ok();
                    }
                    tmpfile.seek(SeekFrom::Start(0)).unwrap();
                    let mut buf = vec![];
                    tmpfile.read_to_end(&mut buf).unwrap();

                    let filename = format!("rnacos_config_export_{}.zip", now_millis());
                    HttpResponse::Ok()
                        .insert_header(header::ContentType::octet_stream())
                        .insert_header(header::ContentDisposition::attachment(filename))
                        .body(buf)
                }
                _ => HttpResponse::InternalServerError().body("config result error"),
            }
        }
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}
