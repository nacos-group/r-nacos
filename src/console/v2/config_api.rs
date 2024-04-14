use crate::common::appdata::AppShareData;
use crate::common::model::{ApiResult, PageResult};
use crate::config::core::{ConfigActor, ConfigCmd, ConfigResult};
use crate::console::model::config_model::{ConfigInfo, ConfigParams, OpsConfigQueryListRequest};
use actix::Addr;
use actix_web::web::Data;
use actix_web::{web, HttpResponse, Responder};
use std::sync::Arc;

pub use crate::console::config_api::{download_config, import_config};
use crate::console::v2::ERROR_CODE_SYSTEM_ERROR;
use crate::raft::cluster::model::{DelConfigReq, SetConfigReq};

pub async fn query_config_list(
    request: web::Query<OpsConfigQueryListRequest>,
    config_addr: web::Data<Addr<ConfigActor>>,
) -> impl Responder {
    let cmd = ConfigCmd::QueryPageInfo(Box::new(request.0.to_param().unwrap()));
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
    web::Query(param): web::Query<ConfigParams>,
    appdata: Data<Arc<AppShareData>>,
) -> impl Responder {
    let config_key = param.to_key();
    let cmd = ConfigCmd::GET(config_key);
    if let Ok(Ok(ConfigResult::DATA(v, md5))) = appdata.config_addr.send(cmd).await {
        HttpResponse::Ok().json(ApiResult::success(Some(ConfigInfo {
            value: Some(v),
            md5: Some(md5),
        })))
    } else {
        HttpResponse::Ok().json(ApiResult::<()>::error(
            ERROR_CODE_SYSTEM_ERROR.to_string(),
            None,
        ))
    }
}

pub async fn add_config(
    appdata: Data<Arc<AppShareData>>,
    web::Json(param): web::Json<ConfigParams>,
) -> impl Responder {
    let content = param.content.clone().unwrap_or_default();
    let config_key = param.to_key();
    let req = SetConfigReq::new(config_key, content);
    if let Ok(_) = appdata.config_route.set_config(req).await {
        HttpResponse::Ok().json(ApiResult::success(Some(true)))
    } else {
        HttpResponse::Ok().json(ApiResult::<()>::error(
            ERROR_CODE_SYSTEM_ERROR.to_string(),
            None,
        ))
    }
}

pub async fn remove_config(
    appdata: Data<Arc<AppShareData>>,
    web::Json(param): web::Json<ConfigParams>,
) -> impl Responder {
    let config_key = param.to_key();
    let req = DelConfigReq::new(config_key);
    if let Ok(_) = appdata.config_route.del_config(req).await {
        HttpResponse::Ok().json(ApiResult::success(Some(true)))
    } else {
        HttpResponse::Ok().json(ApiResult::<()>::error(
            ERROR_CODE_SYSTEM_ERROR.to_string(),
            None,
        ))
    }
}
