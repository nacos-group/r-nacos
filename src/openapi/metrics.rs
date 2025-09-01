use crate::common::appdata::AppShareData;
use crate::metrics::model::{MetricsRequest, MetricsResponse};
use actix_web::{web, HttpResponse, Responder};
use std::sync::Arc;

pub(crate) async fn metrics_info(appdata: web::Data<Arc<AppShareData>>) -> impl Responder {
    if let Ok(Ok(v)) = appdata.metrics_manager.send(MetricsRequest::Export).await {
        match v {
            MetricsResponse::ExportInfo(v) => HttpResponse::Ok()
                .content_type("text/plain;version=0.0.4;charset=utf-8")
                .body(v),
            _ => HttpResponse::InternalServerError().body("metrics module disable"),
        }
    } else {
        HttpResponse::InternalServerError().body("request metrics_manager error")
    }
}

pub fn metrics_config(config: &mut web::ServiceConfig) {
    config
        .service(web::resource("/metrics").route(web::get().to(metrics_info)))
        .service(web::resource("/nacos/metrics").route(web::get().to(metrics_info)))
        .service(web::resource("/rnacos/metrics").route(web::get().to(metrics_info)));
}
