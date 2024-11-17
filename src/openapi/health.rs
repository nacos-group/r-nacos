use crate::common::appdata::AppShareData;
use crate::health::model::{CheckHealthResult, HealthManagerRequest, HealthManagerResponse};
use actix_web::{web, HttpResponse, Responder};
use std::sync::Arc;

pub(crate) async fn health_info(appdata: web::Data<Arc<AppShareData>>) -> impl Responder {
    if let Ok(Ok(HealthManagerResponse::StatusResult(v))) = appdata
        .health_manager
        .send(HealthManagerRequest::Status)
        .await
    {
        match v {
            CheckHealthResult::Success => HttpResponse::Ok().body("success"),
            CheckHealthResult::Error(msg) => {
                HttpResponse::ServiceUnavailable().body(format!("error: {}", msg))
            }
        }
    } else {
        HttpResponse::InternalServerError().body("request health_manager error")
    }
}

pub fn health_config(config: &mut web::ServiceConfig) {
    config
        .service(web::resource("/health").route(web::get().to(health_info)))
        .service(web::resource("/nacos/health").route(web::get().to(health_info)))
        .service(web::resource("/rnacos/health").route(web::get().to(health_info)));
}
