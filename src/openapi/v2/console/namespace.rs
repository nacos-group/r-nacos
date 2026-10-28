use crate::common::appdata::AppShareData;
use crate::common::string_utils::StringUtils;
use crate::console::model::NamespaceInfo;
use crate::console::NamespaceUtils;
use crate::merge_web_param;
use crate::openapi::v1::console::namespace::{NamespaceParam, NamespaceVO};
use crate::openapi::v2::model::ApiResult;
use crate::{token_no_namespace_permission, user_namespace_privilege};
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse, Responder};
use std::sync::Arc;
use uuid::Uuid;

pub async fn query_namespace_list(
    req: HttpRequest,
    app_data: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    let namespace_privilege = user_namespace_privilege!(req);
    let namespaces = match NamespaceUtils::get_namespaces(&app_data).await {
        Ok(namespaces) => namespaces,
        Err(e) => {
            return HttpResponse::InternalServerError().json(ApiResult::server_error(e.to_string()))
        }
    };
    let namespaces = if namespace_privilege.is_all() {
        namespaces
    } else {
        namespaces
            .into_iter()
            .filter(|e| namespace_privilege.check_option_value_permission(&e.namespace_id, false))
            .collect()
    };
    let list: Vec<NamespaceVO> = namespaces.iter().map(|e| e.clone().into()).collect();
    HttpResponse::Ok().json(ApiResult::success(list))
}

pub async fn query_namespace(
    req: HttpRequest,
    web::Query(param): web::Query<NamespaceParam>,
    payload: web::Payload,
    app_data: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    let param = merge_web_param!(param, payload);
    let namespace_privilege = user_namespace_privilege!(req);
    if !namespace_privilege.check_option_value_permission(&param.namespace_id, false) {
        token_no_namespace_permission!(&param.namespace_id);
    }
    match NamespaceUtils::get_namespace(&app_data, param.namespace_id).await {
        Ok(namespace) => {
            let namespace: NamespaceVO = namespace.into();
            HttpResponse::Ok().json(ApiResult::success(namespace))
        }
        Err(e) => HttpResponse::InternalServerError().json(ApiResult::server_error(e.to_string())),
    }
}

pub async fn add_namespace(
    req: HttpRequest,
    web::Query(param): web::Query<NamespaceParam>,
    payload: web::Payload,
    app_data: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    let param = merge_web_param!(param, payload);
    let mut param: NamespaceInfo = param.into();
    if StringUtils::is_option_empty_arc(&param.namespace_id) {
        param.namespace_id = Some(Arc::new(Uuid::new_v4().to_string()));
    }
    let namespace_privilege = user_namespace_privilege!(req);
    if !namespace_privilege.check_option_value_permission(&param.namespace_id, false) {
        token_no_namespace_permission!(&param.namespace_id);
    }
    match NamespaceUtils::add_namespace(&app_data, param).await {
        Ok(_) => HttpResponse::Ok().json(ApiResult::success(true)),
        Err(e) => HttpResponse::InternalServerError().json(ApiResult::server_error(e.to_string())),
    }
}

pub async fn update_namespace(
    req: HttpRequest,
    web::Query(param): web::Query<NamespaceParam>,
    payload: web::Payload,
    app_data: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    let param = merge_web_param!(param, payload);
    let param: NamespaceInfo = param.into();
    let namespace_privilege = user_namespace_privilege!(req);
    if !namespace_privilege.check_option_value_permission(&param.namespace_id, false) {
        token_no_namespace_permission!(&param.namespace_id);
    }
    match NamespaceUtils::update_namespace(&app_data, param).await {
        Ok(_) => HttpResponse::Ok().json(ApiResult::success(true)),
        Err(e) => HttpResponse::InternalServerError().json(ApiResult::server_error(e.to_string())),
    }
}

pub async fn remove_namespace(
    req: HttpRequest,
    web::Query(param): web::Query<NamespaceParam>,
    payload: web::Payload,
    app_data: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    let param = merge_web_param!(param, payload);
    let namespace_privilege = user_namespace_privilege!(req);
    if !namespace_privilege.check_option_value_permission(&param.namespace_id, false) {
        token_no_namespace_permission!(&param.namespace_id);
    }
    match NamespaceUtils::remove_namespace(&app_data, param.namespace_id).await {
        Ok(_) => HttpResponse::Ok().json(ApiResult::success(true)),
        Err(e) => HttpResponse::InternalServerError().json(ApiResult::server_error(e.to_string())),
    }
}
