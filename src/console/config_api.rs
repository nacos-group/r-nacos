#![allow(unused_imports)]

use std::io::{Write, self};
use std::sync::Arc;

use actix_multipart::Multipart;
use actix_multipart::form::MultipartForm;
use actix_multipart::form::tempfile::TempFile;
use actix_web::{
    web,Error, HttpRequest, HttpResponse, Responder, http::header,
};

use actix::prelude::{
    Addr,
};
use tokio_stream::StreamExt;
use uuid::Uuid;
use zip::ZipArchive;
use crate::config::ConfigUtils;
use crate::config::config::{ConfigActor, ConfigCmd, ConfigResult, ConfigKey};
use crate::console::model::config_model::{OpsConfigQueryListRequest, OpsConfigOptQueryListResponse};

use super::model::config_model::OpsConfigImportInfo;

pub async fn query_config_list(request: web::Query<OpsConfigQueryListRequest>, config_addr: web::Data<Addr<ConfigActor>>) -> impl Responder {
    let cmd = ConfigCmd::QueryPageInfo(Box::new(request.0.to_param().unwrap()));
    match config_addr.send(cmd).await {
        Ok(res) => {
            let r: ConfigResult = res.unwrap();
            match r {
                ConfigResult::ConfigInfoPage(size, list) => {
                    let response = OpsConfigOptQueryListResponse {
                        count: size as u64,
                        list
                    };
                    let v = serde_json::to_string(&response).unwrap();
                    HttpResponse::Ok()
                        .insert_header(header::ContentType(mime::APPLICATION_JSON))
                        .body(v)
                }
                _ => {
                    HttpResponse::InternalServerError().body("config result error")
                }
            }
        }
        Err(err) => {
            HttpResponse::InternalServerError().body(err.to_string())
        }
    }
}

#[derive(Debug, MultipartForm)]
pub struct UploadForm {
    #[multipart(rename = "file")]
    pub files: Vec<TempFile>,
}


pub async fn import_config(
    MultipartForm(form): MultipartForm<UploadForm>,
    config_info: web::Form<OpsConfigImportInfo>,
    config_addr:web::Data<Addr<ConfigActor>>
) -> Result<impl Responder, Error> {
    let tenant = Arc::new(ConfigUtils::default_tenant(config_info.0.tenant.unwrap_or_default()));
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
                        let parts =filename.split('/').into_iter().collect::<Vec<_>>();
                        if parts.len()!=2 {
                            continue;
                        }
                        assert!(parts.len()==2);
                        let config_key =  ConfigKey::new_by_arc(Arc::new(parts[1].to_owned()) , Arc::new(parts[0].to_owned()), tenant.clone());
                        let value = match io::read_to_string(&mut file) {
                            Ok(v) => v,
                            Err(_) => continue,
                        };
                        config_addr.do_send(ConfigCmd::ADD(config_key,Arc::new(value)));
                    }
                }
            },
            Err(_) => todo!(),
        }
    }
    Ok(HttpResponse::Ok())
}
