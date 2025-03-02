use crate::common::appdata::AppShareData;
use crate::common::string_utils::StringUtils;
use crate::console::model::NamespaceInfo;
use crate::console::NamespaceUtils;
use crate::merge_web_param;
use crate::openapi::v1::console::namespace::{NamespaceParam, NamespaceVO};
use crate::openapi::v2::model::ApiResult;
use actix_web::{web, HttpResponse, Responder};
use std::sync::Arc;
use uuid::Uuid;

pub async fn query_namespace_list(app_data: web::Data<Arc<AppShareData>>) -> impl Responder {
    let namespaces = match NamespaceUtils::get_namespaces(&app_data).await {
        Ok(namespaces) => namespaces,
        Err(e) => {
            return HttpResponse::InternalServerError().json(ApiResult::server_error(e.to_string()))
        }
    };
    let list: Vec<NamespaceVO> = namespaces.iter().map(|e| e.clone().into()).collect();
    HttpResponse::Ok().json(ApiResult::success(list))
}

pub async fn query_namespace(
    web::Query(param): web::Query<NamespaceParam>,
    payload: web::Payload,
    app_data: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    let param = merge_web_param!(param, payload);
    match NamespaceUtils::get_namespace(&app_data, param.namespace_id).await {
        Ok(namespace) => {
            let namespace: NamespaceVO = namespace.into();
            HttpResponse::Ok().json(ApiResult::success(namespace))
        }
        Err(e) => HttpResponse::InternalServerError().json(ApiResult::server_error(e.to_string())),
    }
}

pub async fn add_namespace(
    web::Query(param): web::Query<NamespaceParam>,
    payload: web::Payload,
    app_data: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    let param = merge_web_param!(param, payload);
    let mut param: NamespaceInfo = param.into();
    if StringUtils::is_option_empty_arc(&param.namespace_id) {
        param.namespace_id = Some(Arc::new(Uuid::new_v4().to_string()));
    }
    match NamespaceUtils::add_namespace(&app_data, param).await {
        Ok(_) => HttpResponse::Ok().json(ApiResult::success(true)),
        Err(e) => HttpResponse::InternalServerError().json(ApiResult::server_error(e.to_string())),
    }
}

pub async fn update_namespace(
    web::Query(param): web::Query<NamespaceParam>,
    payload: web::Payload,
    app_data: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    let param = merge_web_param!(param, payload);
    match NamespaceUtils::update_namespace(&app_data, param.into()).await {
        Ok(_) => HttpResponse::Ok().json(ApiResult::success(true)),
        Err(e) => HttpResponse::InternalServerError().json(ApiResult::server_error(e.to_string())),
    }
}

pub async fn remove_namespace(
    web::Query(param): web::Query<NamespaceParam>,
    payload: web::Payload,
    app_data: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    let param = merge_web_param!(param, payload);
    match NamespaceUtils::remove_namespace(&app_data, param.namespace_id).await {
        Ok(_) => HttpResponse::Ok().json(ApiResult::success(true)),
        Err(e) => HttpResponse::InternalServerError().json(ApiResult::server_error(e.to_string())),
    }
}
