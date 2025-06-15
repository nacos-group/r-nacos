use crate::common::appdata::AppShareData;
use crate::common::model::{ApiResult, PageResult};
use crate::console::model::naming_model::{
    InstanceParams, ServiceDto, ServiceParam, ServiceQueryListRequest,
};
use crate::console::v2::ERROR_CODE_SYSTEM_ERROR;
use crate::grpc::handler::NAMING_ROUTE_REQUEST;
use crate::grpc::PayloadUtils;
use crate::naming::api_model::InstanceVO;
use crate::naming::cluster::model::{NamingRouteRequest, NamingRouterResponse};
use crate::naming::core::{NamingActor, NamingCmd, NamingResult};
use crate::naming::model::{InstanceUpdateTag, ServiceDetailDto};
use crate::naming::service::SubscriberInfoDto;
use crate::naming::service_index::ServiceQueryParam;
use crate::naming::NamingUtils;
use crate::{user_namespace_privilege, user_no_namespace_permission};
use actix::Addr;
use actix_web::web::Data;
use actix_web::HttpMessage;
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use std::sync::Arc;

pub async fn query_service_list(
    req: HttpRequest,
    param: web::Query<ServiceQueryListRequest>,
    naming_addr: web::Data<Addr<NamingActor>>,
) -> impl Responder {
    let service_param = param.0.to_param(&req).unwrap();
    if !service_param
        .namespace_privilege
        .check_option_value_permission(&service_param.namespace_id, true)
    {
        user_no_namespace_permission!(&service_param.namespace_id);
    }
    match naming_addr
        .send(NamingCmd::QueryServiceInfoPage(service_param))
        .await
    {
        Ok(res) => {
            let result: NamingResult = res.unwrap();
            if let NamingResult::ServiceInfoPage((total_count, list)) = result {
                let service_list: Vec<ServiceDto> =
                    list.into_iter().map(ServiceDto::from).collect::<_>();
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
    req: HttpRequest,
    appdata: Data<Arc<AppShareData>>,
    web::Json(param): web::Json<ServiceParam>,
) -> impl Responder {
    let service_key = param.to_key();
    let namespace_privilege = user_namespace_privilege!(req);
    if !namespace_privilege.check_permission(&service_key.namespace_id) {
        user_no_namespace_permission!(&service_key.namespace_id);
    }
    let metadata = if let Some(metadata_str) = param.metadata {
        match NamingUtils::parse_metadata(&metadata_str) {
            Ok(metadata) => Some(Arc::new(metadata)),
            Err(_) => None,
        }
    } else {
        None
    };
    let service_info = ServiceDetailDto {
        namespace_id: service_key.namespace_id,
        service_name: service_key.service_name,
        group_name: service_key.group_name,
        metadata,
        protect_threshold: param.protect_threshold,
        ..Default::default()
    };
    if let Ok(res) = appdata
        .naming_addr
        .send(NamingCmd::UpdateService(service_info))
        .await
    {
        match res {
            Ok(_) => HttpResponse::Ok().json(ApiResult::success(Some(true))),
            Err(err) => HttpResponse::Ok().json(ApiResult::<()>::error(
                ERROR_CODE_SYSTEM_ERROR.to_string(),
                Some(err.to_string()),
            )),
        }
    } else {
        HttpResponse::Ok().json(ApiResult::<()>::error(
            ERROR_CODE_SYSTEM_ERROR.to_string(),
            None,
        ))
    }
}

pub async fn remove_service(
    req: HttpRequest,
    appdata: Data<Arc<AppShareData>>,
    web::Json(param): web::Json<ServiceParam>,
) -> impl Responder {
    let service_key = param.to_key();
    let namespace_privilege = user_namespace_privilege!(req);
    if !namespace_privilege.check_permission(&service_key.namespace_id) {
        user_no_namespace_permission!(&service_key.namespace_id);
    }
    if let Ok(res) = appdata
        .naming_addr
        .send(NamingCmd::RemoveService(service_key))
        .await
    {
        match res {
            Ok(_) => HttpResponse::Ok().json(ApiResult::success(Some(true))),
            Err(err) => HttpResponse::Ok().json(ApiResult::<()>::error(
                ERROR_CODE_SYSTEM_ERROR.to_string(),
                Some(err.to_string()),
            )),
        }
    } else {
        HttpResponse::Ok().json(ApiResult::<()>::error(
            ERROR_CODE_SYSTEM_ERROR.to_string(),
            None,
        ))
    }
}

pub async fn query_instances_list(
    req: HttpRequest,
    param: web::Query<ServiceParam>,
    appdata: Data<Arc<AppShareData>>,
) -> impl Responder {
    let service_key = param.to_key();
    let namespace_privilege = user_namespace_privilege!(req);
    if !namespace_privilege.check_permission(&service_key.namespace_id) {
        user_no_namespace_permission!(&service_key.namespace_id);
    }
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

pub async fn get_instance(
    req: HttpRequest,
    appdata: Data<Arc<AppShareData>>,
    web::Query(param): web::Query<InstanceParams>,
) -> impl Responder {
    match param.to_instance() {
        Ok(instance) => {
            let namespace_privilege = user_namespace_privilege!(req);
            if !namespace_privilege.check_permission(&instance.namespace_id) {
                user_no_namespace_permission!(&instance.namespace_id);
            }
            match appdata.naming_addr.send(NamingCmd::Query(instance)).await {
                Ok(res) => {
                    let result: NamingResult = res.unwrap();
                    match result {
                        NamingResult::Instance(v) => {
                            let vo = InstanceVO::from_instance(&v);
                            HttpResponse::Ok().json(ApiResult::success(Some(vo)))
                        }
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
        Err(err) => HttpResponse::Ok().json(ApiResult::<()>::error(
            ERROR_CODE_SYSTEM_ERROR.to_string(),
            Some(err.to_string()),
        )),
    }
}
pub async fn add_instance(
    req: HttpRequest,
    appdata: Data<Arc<AppShareData>>,
    web::Json(param): web::Json<InstanceParams>,
) -> impl Responder {
    let update_tag = InstanceUpdateTag {
        weight: match &param.weight {
            Some(_v) => true,
            None => false,
        },
        metadata: match &param.metadata {
            Some(_v) => true,
            None => false,
        },
        enabled: match &param.enabled {
            Some(_v) => true,
            None => false,
        },
        ephemeral: match &param.ephemeral {
            Some(_v) => true,
            None => false,
        },
        from_update: true,
    };
    match param.to_instance() {
        Ok(instance) => {
            let namespace_privilege = user_namespace_privilege!(req);
            if !namespace_privilege.check_permission(&instance.namespace_id) {
                user_no_namespace_permission!(&instance.namespace_id);
            }
            if !instance.check_valid() {
                HttpResponse::Ok().json(ApiResult::<()>::error(
                    ERROR_CODE_SYSTEM_ERROR.to_string(),
                    Some("instance check is invalid".to_string()),
                ))
            } else {
                match appdata
                    .naming_route
                    .update_instance(instance, Some(update_tag))
                    .await
                {
                    Ok(_) => HttpResponse::Ok().json(ApiResult::success(Some(true))),
                    Err(err) => HttpResponse::Ok().json(ApiResult::<()>::error(
                        ERROR_CODE_SYSTEM_ERROR.to_string(),
                        Some(err.to_string()),
                    )),
                }
            }
        }
        Err(err) => HttpResponse::Ok().json(ApiResult::<()>::error(
            ERROR_CODE_SYSTEM_ERROR.to_string(),
            Some(err.to_string()),
        )),
    }
}

pub async fn remove_instance(
    req: HttpRequest,
    appdata: Data<Arc<AppShareData>>,
    web::Json(param): web::Json<InstanceParams>,
) -> impl Responder {
    match param.to_instance() {
        Ok(instance) => {
            let namespace_privilege = user_namespace_privilege!(req);
            if !namespace_privilege.check_permission(&instance.namespace_id) {
                user_no_namespace_permission!(&instance.namespace_id);
            }
            if !instance.check_valid() {
                HttpResponse::Ok().json(ApiResult::<()>::error(
                    ERROR_CODE_SYSTEM_ERROR.to_string(),
                    Some("instance check is invalid".to_string()),
                ))
            } else {
                match appdata.naming_route.delete_instance(instance).await {
                    Ok(_) => HttpResponse::Ok().json(ApiResult::success(Some(true))),
                    Err(err) => HttpResponse::Ok().json(ApiResult::<()>::error(
                        ERROR_CODE_SYSTEM_ERROR.to_string(),
                        Some(err.to_string()),
                    )),
                }
            }
        }
        Err(err) => HttpResponse::Ok().json(ApiResult::<()>::error(
            ERROR_CODE_SYSTEM_ERROR.to_string(),
            Some(err.to_string()),
        )),
    }
}

pub async fn query_subscribers_list(
    app: Data<Arc<AppShareData>>,
    req: HttpRequest,
    param: web::Query<ServiceQueryListRequest>,
) -> impl Responder {
    let node_id = param.node_id.unwrap_or(0);
    let service_param = param.0.to_param(&req).unwrap();
    if !service_param
        .namespace_privilege
        .check_option_value_permission(&service_param.namespace_id, true)
    {
        user_no_namespace_permission!(&service_param.namespace_id);
    }
    match do_query_subscribers_list(app, service_param, node_id).await {
        Ok(res) => HttpResponse::Ok().json(ApiResult::success(Some(res))),
        Err(err) => HttpResponse::Ok().json(ApiResult::<()>::error(
            ERROR_CODE_SYSTEM_ERROR.to_string(),
            Some(err.to_string()),
        )),
    }
}

async fn do_query_subscribers_list(
    app: Data<Arc<AppShareData>>,
    param: ServiceQueryParam,
    node_id: u64,
) -> anyhow::Result<PageResult<SubscriberInfoDto>> {
    if node_id == 0 || node_id == app.sys_config.raft_node_id {
        if let NamingResult::ServiceSubscribersPage((total_count, list)) = app
            .naming_addr
            .send(NamingCmd::QueryServiceSubscribersPageV2(param))
            .await??
        {
            Ok(PageResult { total_count, list })
        } else {
            Err(anyhow::anyhow!("query subscribers error"))
        }
    } else {
        let addr = app.naming_node_manage.get_node_addr(node_id).await?;
        let req = NamingRouteRequest::QueryServiceSubscriberPage(param);
        let request = serde_json::to_string(&req).unwrap_or_default();
        let payload = PayloadUtils::build_payload(NAMING_ROUTE_REQUEST, request);
        let resp_payload = app
            .cluster_sender
            .send_request(addr.clone(), payload)
            .await?;
        let body_vec = resp_payload.body.unwrap_or_default().value;
        let resp: NamingRouterResponse = serde_json::from_slice(&body_vec)?;
        if let NamingRouterResponse::ServiceSubscribersPage((total_count, list)) = resp {
            Ok(PageResult { total_count, list })
        } else {
            Err(anyhow::anyhow!("query subscribers error"))
        }
    }
}
