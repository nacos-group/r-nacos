use crate::common::appdata::AppShareData;
use crate::common::error_code::NO_PERMISSION;
use crate::common::model::ApiResult;
use crate::common::string_utils::StringUtils;
use crate::console::model::NamespaceInfo;
use crate::console::NamespaceUtils;
use crate::user_namespace_privilege;
use actix_http::HttpMessage;
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use std::sync::Arc;
use uuid::Uuid;

pub async fn query_namespace_list(
    req: HttpRequest,
    app_data: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    let namespace_privilege = user_namespace_privilege!(req);
    let namespaces = NamespaceUtils::get_namespaces(&app_data)
        .await
        .unwrap_or_default();
    let namespaces = if namespace_privilege.is_all() {
        namespaces
    } else {
        namespaces
            .into_iter()
            .filter(|e| namespace_privilege.check_option_value_permission(&e.namespace_id, false))
            .collect()
    };
    HttpResponse::Ok().json(ApiResult::success(Some(namespaces)))
}

pub async fn add_namespace(
    req: HttpRequest,
    param: web::Json<NamespaceInfo>,
    app_data: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    let mut param = param.0;
    if StringUtils::is_option_empty_arc(&param.namespace_id) {
        param.namespace_id = Some(Arc::new(Uuid::new_v4().to_string()));
    }
    let namespace_privilege = user_namespace_privilege!(req);
    if !namespace_privilege.check_option_value_permission(&param.namespace_id, false) {
        return HttpResponse::Ok().json(ApiResult::<()>::error(
            NO_PERMISSION.to_string(),
            Some(format!(
                "user no such namespace permission: {:?}",
                &param.namespace_id
            )),
        ));
    }
    match NamespaceUtils::add_namespace(&app_data, param).await {
        Ok(_) => HttpResponse::Ok().json(ApiResult::success(Some(true))),
        Err(e) => HttpResponse::Ok().json(ApiResult::<()>::error(
            "SYSTEM_ERROR".to_string(),
            Some(e.to_string()),
        )),
    }
}

pub async fn update_namespace(
    req: HttpRequest,
    param: web::Json<NamespaceInfo>,
    app_data: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    let namespace_privilege = user_namespace_privilege!(req);
    if !namespace_privilege.check_option_value_permission(&param.namespace_id, false) {
        return HttpResponse::Ok().json(ApiResult::<()>::error(
            NO_PERMISSION.to_string(),
            Some(format!(
                "user no such namespace permission: {:?}",
                &param.namespace_id
            )),
        ));
    }
    match NamespaceUtils::update_namespace(&app_data, param.0).await {
        Ok(_) => HttpResponse::Ok().json(ApiResult::success(Some(true))),
        Err(e) => HttpResponse::Ok().json(ApiResult::<()>::error(
            "SYSTEM_ERROR".to_string(),
            Some(e.to_string()),
        )),
    }
}

pub async fn remove_namespace(
    req: HttpRequest,
    param: web::Json<NamespaceInfo>,
    app_data: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    let namespace_privilege = user_namespace_privilege!(req);
    if !namespace_privilege.check_option_value_permission(&param.namespace_id, false) {
        return HttpResponse::Ok().json(ApiResult::<()>::error(
            NO_PERMISSION.to_string(),
            Some(format!(
                "user no such namespace permission: {:?}",
                &param.namespace_id
            )),
        ));
    }
    match NamespaceUtils::remove_namespace(&app_data, param.0.namespace_id).await {
        Ok(_) => HttpResponse::Ok().json(ApiResult::success(Some(true))),
        Err(e) => HttpResponse::Ok().json(ApiResult::<()>::error(
            "SYSTEM_ERROR".to_string(),
            Some(e.to_string()),
        )),
    }
}
