use crate::common::appdata::AppShareData;
use crate::common::model::{ApiResult, PageResult};
use crate::console::model::naming_model::{
    OpsNamingQueryListResponse, QueryAllInstanceListParam, ServiceParam,
};
use crate::console::v2::ERROR_CODE_SYSTEM_ERROR;
use crate::naming::core::{NamingActor, NamingCmd, NamingResult};
use crate::naming::model::ServiceDetailDto;
use crate::naming::ops::ops_model::{OpsServiceDto, OpsServiceQueryListRequest};
use actix::Addr;
use actix_web::http::header;
use actix_web::web::Data;
use actix_web::{web, HttpResponse, Responder};
use std::sync::Arc;

pub async fn query_service_list(
    param: web::Query<OpsServiceQueryListRequest>,
    naming_addr: web::Data<Addr<NamingActor>>,
) -> impl Responder {
    let service_param = param.0.to_param().unwrap();
    match naming_addr
        .send(NamingCmd::QueryServiceInfoPage(service_param))
        .await
    {
        Ok(res) => {
            let result: NamingResult = res.unwrap();
            if let NamingResult::ServiceInfoPage((total_count, list)) = result {
                let service_list: Vec<OpsServiceDto> =
                    list.into_iter().map(OpsServiceDto::from).collect::<_>();
                HttpResponse::Ok().json(ApiResult::success(Some(PageResult {
                    total_count,
                    list: service_list,
                })))
            } else {
                HttpResponse::Ok().json(ApiResult::<()>::error(
                    ERROR_CODE_SYSTEM_ERROR.to_string(),
                    None,
                ))
            }
        }
        Err(err) => HttpResponse::Ok().json(ApiResult::<()>::error(
            ERROR_CODE_SYSTEM_ERROR.to_string(),
            Some(err.to_string()),
        )),
    }
}

pub async fn add_service(
    appdata: Data<Arc<AppShareData>>,
    web::Json(param): web::Json<ServiceParam>,
) -> impl Responder {
    let service_key = param.to_key();
    let service_info = ServiceDetailDto {
        namespace_id: service_key.namespace_id,
        service_name: service_key.service_name,
        group_name: service_key.group_name,
        metadata: param.metadata,
        protect_threshold: param.protect_threshold,
    };
    if let Ok(_) = appdata
        .naming_addr
        .send(NamingCmd::UpdateService(service_info))
        .await
    {
        HttpResponse::Ok().json(ApiResult::success(Some(true)))
    } else {
        HttpResponse::Ok().json(ApiResult::<()>::error(
            ERROR_CODE_SYSTEM_ERROR.to_string(),
            None,
        ))
    }
}

pub async fn remove_service(
    appdata: Data<Arc<AppShareData>>,
    web::Json(param): web::Json<ServiceParam>,
) -> impl Responder {
    let service_key = param.to_key();
    if let Ok(_) = appdata
        .naming_addr
        .send(NamingCmd::RemoveService(service_key))
        .await
    {
        HttpResponse::Ok().json(ApiResult::success(Some(true)))
    } else {
        HttpResponse::Ok().json(ApiResult::<()>::error(
            ERROR_CODE_SYSTEM_ERROR.to_string(),
            None,
        ))
    }
}

pub async fn query_instances_list(
    param: web::Query<ServiceParam>,
    appdata: Data<Arc<AppShareData>>,
) -> impl Responder {
    let service_key = param.to_key();
    match appdata
        .naming_addr
        .send(NamingCmd::QueryAllInstanceList(service_key))
        .await
    {
        Ok(res) => match res as anyhow::Result<NamingResult> {
            Ok(result) => match result {
                NamingResult::InstanceList(list) => {
                    HttpResponse::Ok().json(ApiResult::success(Some(PageResult {
                        total_count: list.len(),
                        list,
                    })))
                }
                _ => HttpResponse::Ok().json(ApiResult::<()>::error(
                    ERROR_CODE_SYSTEM_ERROR.to_string(),
                    None,
                )),
            },
            Err(err) => HttpResponse::Ok().json(ApiResult::<()>::error(
                ERROR_CODE_SYSTEM_ERROR.to_string(),
                Some(err.to_string()),
            )),
        },
        Err(err) => HttpResponse::Ok().json(ApiResult::<()>::error(
            ERROR_CODE_SYSTEM_ERROR.to_string(),
            Some(err.to_string()),
        )),
    }
}
