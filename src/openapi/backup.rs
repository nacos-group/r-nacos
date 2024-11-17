use crate::common::appdata::AppShareData;
use crate::console::transfer_api::download_transfer_file;
use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Deserialize, Serialize)]
pub struct BackupParam {
    pub token: Arc<String>,
}

pub async fn backup(
    app_share_data: web::Data<Arc<AppShareData>>,
    web::Query(params): web::Query<BackupParam>,
) -> impl Responder {
    if app_share_data.sys_config.backup_token.is_empty() {
        HttpResponse::InternalServerError().body("backup api is not open")
    } else if params.token.as_str() != app_share_data.sys_config.backup_token.as_str() {
        HttpResponse::InternalServerError().body("backup token is not matched")
    } else {
        download_transfer_file(app_share_data).await
    }
}

pub fn backup_config(config: &mut web::ServiceConfig) {
    config.service(web::resource("/rnacos/backup").route(web::get().to(backup)));
}
