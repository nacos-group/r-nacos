use crate::common::appdata::AppShareData;
use crate::console::config_api::UploadForm;
use crate::now_millis;
use crate::transfer::model::{
    TransferBackupParam, TransferImportParam, TransferImportResponse,
    TransferManagerAsyncRequest, TransferManagerResponse,
};
use actix_multipart::form::MultipartForm;
use actix_web::http::header;
use actix_web::{error, web, Error, HttpRequest, HttpResponse, Responder};
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

        let filename = format!("rnacos_export_{}.data", now_millis());
        HttpResponse::Ok()
            .insert_header(header::ContentType::octet_stream())
            .insert_header(header::ContentDisposition::attachment(filename))
            .body(buf)
    } else {
        HttpResponse::InternalServerError().body("transfer backup result error")
    }
}

pub async fn import_transfer_file(
    req: HttpRequest,
    MultipartForm(form): MultipartForm<UploadForm>,
    app_share_data: web::Data<Arc<AppShareData>>,
) -> Result<impl Responder, Error> {
    let mut param = TransferImportParam::all();
    if let Some(v) = req.headers().get("import-config") {
        param.config = String::from_utf8_lossy(v.as_bytes()).as_ref() == "1";
    };
    if let Some(v) = req.headers().get("import-user") {
        param.user = String::from_utf8_lossy(v.as_bytes()).as_ref() == "1";
    };
    if let Some(v) = req.headers().get("import-cache") {
        param.cache = String::from_utf8_lossy(v.as_bytes()).as_ref() == "1";
    };
    for mut f in form.files {
        let mut data = Vec::new();
        f.file.read_to_end(&mut data).unwrap();
        let r = app_share_data
            .raft_request_route
            .request_import(data, param.clone())
            .await
            .map_err(error::ErrorInternalServerError)?;
        if let TransferImportResponse::Running = r {
            return Err(error::ErrorInternalServerError(
                "This import is terminated because other imports are not processed",
            ));
        }
    }
    Ok(HttpResponse::Ok())
}
