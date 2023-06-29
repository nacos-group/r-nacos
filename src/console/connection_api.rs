use actix_web::{http::header, web, HttpResponse, Responder};

use actix::prelude::Addr;

use crate::grpc::bistream_manage::{BiStreamManage, BiStreamManageCmd, BiStreamManageResult};

use super::model::PageResult;

pub async fn query_grpc_connection(
    conn_manager_addr: web::Data<Addr<BiStreamManage>>,
) -> impl Responder {
    match conn_manager_addr
        .send(BiStreamManageCmd::QueryConnList)
        .await
    {
        Ok(res) => match res as anyhow::Result<BiStreamManageResult> {
            Ok(result) => match result {
                BiStreamManageResult::ConnList(list) => {
                    let resp = PageResult {
                        count: list.len() as u64,
                        list,
                    };
                    let v = serde_json::to_string(&resp).unwrap();
                    HttpResponse::Ok()
                        .insert_header(header::ContentType(mime::APPLICATION_JSON))
                        .body(v)
                }
                _ => HttpResponse::InternalServerError().body("error result"),
            },
            Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
        },
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}
