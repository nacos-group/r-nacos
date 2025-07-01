use crate::common::appdata::AppShareData;
use crate::common::model::{ApiResult, PageResult, UserSession};
use crate::config::core::{ConfigActor, ConfigCmd, ConfigResult};
pub use crate::console::config_api::{download_config, import_config};
use crate::console::model::config_model::{ConfigInfo, ConfigParams, OpsConfigQueryListRequest};
use crate::console::v2::ERROR_CODE_SYSTEM_ERROR;
use crate::raft::cluster::model::{DelConfigReq, SetConfigReq};
use crate::{user_namespace_privilege, user_no_namespace_permission};
use actix::Addr;
use actix_web::web::Data;
use actix_web::HttpMessage;
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use std::sync::Arc;

pub async fn query_config_list(
    req: HttpRequest,
    request: web::Query<OpsConfigQueryListRequest>,
    config_addr: web::Data<Addr<ConfigActor>>,
) -> impl Responder {
    let param = request.0.to_param(&req).unwrap();
    if !param
        .namespace_privilege
        .check_option_value_permission(&param.tenant, true)
    {
        user_no_namespace_permission!(&param.tenant);
    }
    let cmd = ConfigCmd::QueryPageInfo(Box::new(param));
    match config_addr.send(cmd).await {
        Ok(res) => {
            let r: ConfigResult = res.unwrap();
            match r {
                ConfigResult::ConfigInfoPage(total_count, list) => HttpResponse::Ok()
                    .json(ApiResult::success(Some(PageResult { total_count, list }))),
                _ => HttpResponse::Ok().json(ApiResult::<()>::error(
                    ERROR_CODE_SYSTEM_ERROR.to_string(),
                    None,
                )),
            }
        }
        Err(err) => HttpResponse::Ok().json(ApiResult::<()>::error(
            ERROR_CODE_SYSTEM_ERROR.to_string(),
            Some(err.to_string()),
        )),
    }
}

pub async fn query_history_config_page(
    req: HttpRequest,
    request: web::Query<OpsConfigQueryListRequest>,
    config_addr: web::Data<Addr<ConfigActor>>,
) -> impl Responder {
    let param = match request.0.to_history_param() {
        Ok(param) => param,
        Err(err) => {
            return HttpResponse::Ok().json(ApiResult::<()>::error(
                ERROR_CODE_SYSTEM_ERROR.to_string(),
                Some(err.to_string()),
            ));
        }
    };
    let namespace_privilege = user_namespace_privilege!(req);
    if !namespace_privilege
        .check_option_value_permission(&(param.tenant.clone().map(Arc::new)), false)
    {
        user_no_namespace_permission!(&param.tenant);
    }
    let cmd = ConfigCmd::QueryHistoryPageInfo(Box::new(param));
    match config_addr.send(cmd).await {
        Ok(res) => {
            let r: ConfigResult = res.unwrap();
            match r {
                ConfigResult::ConfigHistoryInfoPage(total_count, list) => HttpResponse::Ok()
                    .json(ApiResult::success(Some(PageResult { total_count, list }))),
                _ => HttpResponse::Ok().json(ApiResult::<()>::error(
                    ERROR_CODE_SYSTEM_ERROR.to_string(),
                    None,
                )),
            }
        }
        Err(err) => HttpResponse::Ok().json(ApiResult::<()>::error(
            ERROR_CODE_SYSTEM_ERROR.to_string(),
            Some(err.to_string()),
        )),
    }
}

pub(crate) async fn get_config(
    req: HttpRequest,
    web::Query(param): web::Query<ConfigParams>,
    appdata: Data<Arc<AppShareData>>,
) -> impl Responder {
    let config_key = param.to_key();
    let namespace_privilege = user_namespace_privilege!(req);
    if !namespace_privilege.check_permission(&config_key.tenant) {
        user_no_namespace_permission!(&config_key.tenant);
    }
    let cmd = ConfigCmd::GET(config_key);
    if let Ok(Ok(ConfigResult::Data {
        value: v,
        md5,
        config_type,
        desc,
        ..
    })) = appdata.config_addr.send(cmd).await
    {
        HttpResponse::Ok().json(ApiResult::success(Some(ConfigInfo {
            value: Some(v),
            md5: Some(md5),
            config_type,
            desc,
        })))
    } else {
        HttpResponse::Ok().json(ApiResult::<()>::error(
            ERROR_CODE_SYSTEM_ERROR.to_string(),
            None,
        ))
    }
}

pub async fn add_config(
    req: HttpRequest,
    appdata: Data<Arc<AppShareData>>,
    web::Json(param): web::Json<ConfigParams>,
) -> impl Responder {
    let op_user = req
        .extensions()
        .get::<Arc<UserSession>>()
        .map(|session| session.username.clone());

    let content = param.content.clone().unwrap_or_default();
    let config_key = param.to_key();
    let namespace_privilege = user_namespace_privilege!(req);
    if !namespace_privilege.check_permission(&config_key.tenant) {
        user_no_namespace_permission!(&config_key.tenant);
    }
    if let Err(e) = config_key.is_valid() {
        return HttpResponse::Ok().json(ApiResult::<()>::error(
            ERROR_CODE_SYSTEM_ERROR.to_string(),
            Some(e.to_string()),
        ));
    }
    let mut req = SetConfigReq::new(config_key, content);
    req.config_type = param.config_type;
    req.desc = param.desc;
    req.op_user = op_user;
    if appdata.config_route.set_config(req).await.is_ok() {
        HttpResponse::Ok().json(ApiResult::success(Some(true)))
    } else {
        HttpResponse::Ok().json(ApiResult::<()>::error(
            ERROR_CODE_SYSTEM_ERROR.to_string(),
            None,
        ))
    }
}

pub async fn remove_config(
    req: HttpRequest,
    appdata: Data<Arc<AppShareData>>,
    web::Json(param): web::Json<ConfigParams>,
) -> impl Responder {
    let config_key = param.to_key();
    let namespace_privilege = user_namespace_privilege!(req);
    if !namespace_privilege.check_permission(&config_key.tenant) {
        user_no_namespace_permission!(&config_key.tenant);
    }
    let req = DelConfigReq::new(config_key);
    if appdata.config_route.del_config(req).await.is_ok() {
        HttpResponse::Ok().json(ApiResult::success(Some(true)))
    } else {
        HttpResponse::Ok().json(ApiResult::<()>::error(
            ERROR_CODE_SYSTEM_ERROR.to_string(),
            None,
        ))
    }
}
