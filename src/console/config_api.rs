#![allow(unused_imports)]

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::sync::Arc;

use actix_multipart::form::tempfile::TempFile;
use actix_multipart::form::text::Text;
use actix_multipart::form::MultipartForm;
use actix_multipart::Multipart;
use actix_web::{error, http::header, web, Error, HttpRequest, HttpResponse, Responder, Result};
use zip::write::FileOptions;

use crate::common::appdata::AppShareData;
use crate::config::core::{
    ConfigActor, ConfigAsyncCmd, ConfigCmd, ConfigInfoDto, ConfigKey, ConfigResult,
};
use crate::config::dal::QueryListeners;
use crate::config::ConfigUtils;
use crate::console::model::config_model::{
    OpsConfigOptQueryListResponse, OpsConfigQueryListRequest,
};
use crate::now_millis;
use crate::raft::cluster::model::SetConfigReq;
use actix::prelude::Addr;
use tokio_stream::StreamExt;
use uuid::Uuid;
use zip::{ZipArchive, ZipWriter};

use super::model::config_model::OpsConfigImportInfo;
use super::model::paginate::{PaginateQuery, PaginateResponse};
use super::model::PageResult;

pub async fn query_config_list(
    request: web::Query<OpsConfigQueryListRequest>,
    config_addr: web::Data<Addr<ConfigActor>>,
) -> impl Responder {
    let cmd = ConfigCmd::QueryPageInfo(Box::new(request.0.to_param().unwrap()));
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
    //let tenant = Arc::new(ConfigUtils::default_tenant(config_info.0.tenant.unwrap_or_default()));
    for f in form.files {
        match zip::ZipArchive::new(f.file) {
            Ok(mut archive) => {
                for i in 0..archive.len() {
                    let mut file = archive.by_index(i).unwrap();
                    /*
                    let filepath = match file.enclosed_name() {
                        Some(path) => path,
                        None => continue,
                    };
                    */
                    let filename = file.name();
                    if !(*filename).ends_with('/') {
                        let parts = filename.split('/').collect::<Vec<_>>();
                        if parts.len() != 2 {
                            continue;
                        }
                        assert!(parts.len() == 2);
                        let config_key = ConfigKey::new_by_arc(
                            Arc::new(parts[1].to_owned()),
                            Arc::new(parts[0].to_owned()),
                            tenant.clone(),
                        );
                        let value = match io::read_to_string(&mut file) {
                            Ok(v) => v,
                            Err(_) => continue,
                        };
                        //println!("update load, {:?}:{}",&config_key,&value);
                        //config_addr.do_send(ConfigAsyncCmd::Add(config_key, Arc::new(value)));
                        let mut req = SetConfigReq::new(config_key.clone(), Arc::new(value));
                        let data_id_clone = config_key.data_id.clone();
                        req.config_type = SetConfigReq::detect_config_type(data_id_clone);

                        app.config_route.set_config(req).await.ok();
                    }
                }
            }
            Err(_) => todo!(),
        }
    }
    Ok(HttpResponse::Ok())
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
            format!("{}/{}", item.group.as_str(), item.data_id.as_str()),
            options,
        )?;
        zip.write_all(item.content.as_ref().unwrap().as_bytes())?;
    }
    zip.finish()?;
    Ok(())
}

///
/// 按查询条件导出配置
pub async fn download_config(
    request: web::Query<OpsConfigQueryListRequest>,
    config_addr: web::Data<Addr<ConfigActor>>,
) -> impl Responder {
    let mut param = request.0.to_param().unwrap();
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

pub async fn query_config_listener_list(
    query: web::Query<PaginateQuery>,
    path: web::Path<String>,
    config_addr: web::Data<Addr<ConfigActor>>,
) -> Result<impl Responder> {
    let config_key = ConfigKey::from(path.as_str());

    config_key.is_valid().map_err(error::ErrorBadRequest)?;

    let cmd = ConfigCmd::QueryListeners(QueryListeners {
        config_key,
        paginate: query.into_inner(),
    });

    let result = config_addr
        .send(cmd)
        .await
        .map_err(error::ErrorInternalServerError)?
        .map_err(error::ErrorInternalServerError)?;

    match result {
        ConfigResult::ConfigListenerInfoPage(count, subscribers) => {
            Ok(web::Json(PaginateResponse {
                count,
                list: subscribers,
            }))
        }
        _ => Err(error::ErrorInternalServerError("config result error")),
    }
}
