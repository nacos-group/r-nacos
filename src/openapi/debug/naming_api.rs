use crate::common::appdata::AppShareData;
use crate::naming::naming_debug::NamingDebugCmd;
use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Deserialize, Serialize)]
pub struct NamingDebugParam {
    pub req_type: String,
}

pub async fn naming_debug_req(
    app_share_data: web::Data<Arc<AppShareData>>,
    web::Query(param): web::Query<NamingDebugParam>,
) -> impl Responder {
    let debug_cmd: Option<NamingDebugCmd> = match param.req_type.as_str() {
        "invalid_local_instance" => Some(NamingDebugCmd::SetLocalInstanceIllHealth),
        "clear_local_http_instance" => Some(NamingDebugCmd::ClearLocalHttpInstance),
        "clear_local_grpc_instance" => Some(NamingDebugCmd::ClearLocalGrpcInstance),
        _ => None,
    };
    if let Some(debug_cmd) = debug_cmd {
        app_share_data.naming_addr.do_send(debug_cmd);
    }
    HttpResponse::Ok().body("ok")
}
