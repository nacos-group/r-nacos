use crate::common::appdata::AppShareData;
use crate::health::model::{CheckHealthResult, HealthManagerRequest, HealthManagerResponse};
use actix_web::{web, HttpResponse, Responder};
use std::sync::Arc;

pub mod namespace;

pub(crate) async fn readiness(appdata: web::Data<Arc<AppShareData>>) -> impl Responder {
    if let Ok(Ok(HealthManagerResponse::StatusResult(v))) = appdata
        .health_manager
        .send(HealthManagerRequest::Status)
        .await
    {
        match v {
            CheckHealthResult::Success => HttpResponse::Ok().body("OK"),
            CheckHealthResult::Error(msg) => {
                HttpResponse::ServiceUnavailable().body(format!("error: {}", msg))
            }
        }
    } else {
        HttpResponse::InternalServerError().body("request health_manager error")
    }
}
