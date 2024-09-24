use crate::common::appdata::AppShareData;
use crate::console::config_api::UploadForm;
use crate::now_millis;
use crate::transfer::model::{
    TransferBackupParam, TransferImportRequest, TransferManagerAsyncRequest,
    TransferManagerResponse,
};
use actix_multipart::form::MultipartForm;
use actix_web::http::header;
use actix_web::{web, Error, HttpResponse, Responder};
use std::io::Read;
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

pub async fn download_transfer_file(
    app_share_data: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    if let Ok(Ok(TransferManagerResponse::BackupFile(temp_file))) = app_share_data
        .transfer_writer_manager
        .send(TransferManagerAsyncRequest::Backup(
            TransferBackupParam::all(),
        ))
        .await
    {
        let mut tmpfile = OpenOptions::new()
            .read(true)
            .open(temp_file.path.clone())
            .await
            .unwrap();
        // Seek to start
        tmpfile.seek(std::io::SeekFrom::Start(0)).await.ok();
        let mut buf = vec![];
        tmpfile.read_to_end(&mut buf).await.ok();

        let filename = format!("rnacos_export_{}.bak", now_millis());
        HttpResponse::Ok()
            .insert_header(header::ContentType::octet_stream())
            .insert_header(header::ContentDisposition::attachment(filename))
            .body(buf)
    } else {
        HttpResponse::InternalServerError().body("transfer backup result error")
    }
}

pub async fn import_transfer_file(
    MultipartForm(form): MultipartForm<UploadForm>,
    app_share_data: web::Data<Arc<AppShareData>>,
) -> Result<impl Responder, Error> {
    for mut f in form.files {
        let mut data = Vec::new();
        f.file.read_to_end(&mut data).unwrap();
        app_share_data
            .transfer_import_manager
            .send(TransferImportRequest::Import(data))
            .await
            .ok();
    }
    Ok(HttpResponse::Ok())
}
