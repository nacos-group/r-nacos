use crate::common::appdata::AppShareData;
use crate::now_millis;
use crate::transfer::model::{
    TransferBackupParam, TransferManagerAsyncRequest, TransferManagerResponse,
};
use actix_web::http::header;
use actix_web::{web, HttpResponse, Responder};
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
