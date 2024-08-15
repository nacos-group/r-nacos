use crate::common::appdata::AppShareData;
use crate::common::model::ApiResult;
//use crate::config::core::ConfigActor;
use crate::console::model::NamespaceInfo;
use crate::console::NamespaceUtils;
//use actix::Addr;
use actix_web::{web, HttpResponse, Responder};
use std::sync::Arc;

pub async fn query_namespace_list(app_data: web::Data<Arc<AppShareData>>) -> impl Responder {
    let namespaces = NamespaceUtils::get_namespaces(&app_data)
        .await
        .unwrap_or_default();
    HttpResponse::Ok().json(ApiResult::success(Some(namespaces)))
}

pub async fn add_namespace(
    param: web::Json<NamespaceInfo>,
    app_data: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    match NamespaceUtils::add_namespace(&app_data, param.0).await {
        Ok(_) => HttpResponse::Ok().json(ApiResult::success(Some(true))),
        Err(e) => HttpResponse::Ok().json(ApiResult::<()>::error(
            "SYSTEM_ERROR".to_string(),
            Some(e.to_string()),
        )),
    }
}

pub async fn update_namespace(
    param: web::Json<NamespaceInfo>,
    app_data: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    match NamespaceUtils::update_namespace(&app_data, param.0).await {
        Ok(_) => HttpResponse::Ok().json(ApiResult::success(Some(true))),
        Err(e) => HttpResponse::Ok().json(ApiResult::<()>::error(
            "SYSTEM_ERROR".to_string(),
            Some(e.to_string()),
        )),
    }
}

pub async fn remove_namespace(
    param: web::Json<NamespaceInfo>,
    app_data: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    match NamespaceUtils::remove_namespace(&app_data, param.0.namespace_id).await {
        Ok(_) => HttpResponse::Ok().json(ApiResult::success(Some(true))),
        Err(e) => HttpResponse::Ok().json(ApiResult::<()>::error(
            "SYSTEM_ERROR".to_string(),
            Some(e.to_string()),
        )),
    }
}
