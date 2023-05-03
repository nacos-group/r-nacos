#![allow(unused_imports)]

use actix_web::{
    web, HttpRequest, HttpResponse, Responder, http::header,
};

use actix::prelude::{
    Addr,
};
use crate::config::config::{ConfigActor, ConfigCmd, ConfigResult};
use crate::console::model::config_model::{OpsConfigQueryListRequest, OpsConfigOptQueryListResponse};

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
