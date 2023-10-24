use std::sync::Arc;

use actix_web::{
    web::{self, Data, Json},
    Responder,
};
use serde::{Deserialize, Serialize};

use crate::common::appdata::AppShareData;

use super::table::{TableManageCmd, TableManageQueryCmd, TableManageResult};

#[derive(Debug, Deserialize)]
pub struct KvOpParam {
    pub table_name: Arc<String>,
    pub key: String,
    pub value: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct KvValueResult {
    pub success: bool,
    pub value: String,
}

pub async fn set(
    app: Data<Arc<AppShareData>>,
    web::Form(param): web::Form<KvOpParam>,
) -> actix_web::Result<impl Responder> {
    app.raft_table_route
        .request(TableManageCmd::Set {
            table_name: param.table_name,
            key: param.key.as_bytes().to_owned(),
            value: param.value.unwrap_or_default().as_bytes().to_owned(),
            last_seq_id: None,
        })
        .await
        .unwrap();
    Ok("{\"ok\":1}")
}

pub async fn get(
    app: Data<Arc<AppShareData>>,
    web::Query(param): web::Query<KvOpParam>,
) -> actix_web::Result<impl Responder> {
    let res: TableManageResult = app
        .raft_table_manage
        .send(TableManageQueryCmd::Get {
            table_name: param.table_name,
            key: param.key.as_bytes().to_owned(),
        })
        .await
        .unwrap()
        .unwrap();
    match res {
        TableManageResult::Value(value) => Ok(Json(KvValueResult {
            value: String::from_utf8(value).unwrap(),
            success: true,
        })),
        _ => Ok(Json(KvValueResult {
            value: "not found key value".to_owned(),
            success: false,
        })),
    }
}
