use std::sync::Arc;

use actix::Addr;
use actix_web::{http::header, web, HttpResponse, Responder};

use crate::common::appdata::AppShareData;
use crate::merge_web_param;
use crate::naming::api_model::InstanceVO;
use crate::naming::core::{NamingActor, NamingCmd, NamingResult};
use crate::naming::model::{InstanceUpdateTag, ServiceKey};
use crate::naming::NamingUtils;
use crate::openapi::naming::model::{
    InstanceWebParams, InstanceWebQueryListParams, ServiceInfoVo, ServiceQueryListRequest,
};
use crate::openapi::v2::model::ApiResult;
use crate::naming::api_model::ServiceInfoParam;
use crate::utils::get_bool_from_string;

// ===== Instance APIs =====

pub async fn get_instance(
    param: web::Query<InstanceWebParams>,
    naming_addr: web::Data<Addr<NamingActor>>,
) -> impl Responder {
    let instance = param.0.convert_to_instance();
    match instance {
        Ok(instance) => match naming_addr.send(NamingCmd::Query(instance)).await {
            Ok(res) => {
                let result: NamingResult = res.unwrap();
                match result {
                    NamingResult::Instance(v) => {
                        let vo = InstanceVO::from_instance(&v);
                        HttpResponse::Ok().json(ApiResult::success(vo))
                    }
                    _ => HttpResponse::Ok().json(ApiResult::<()>::server_error(())),
                }
            }
            Err(e) => {
                HttpResponse::Ok().json(ApiResult::error(30000, e.to_string(), ()))
            }
        },
        Err(e) => {
            HttpResponse::Ok().json(ApiResult::error(30000, e, ()))
        }
    }
}

pub async fn update_instance(
    param: web::Query<InstanceWebParams>,
    payload: web::Payload,
    appdata: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    let param = merge_web_param!(param.0, payload);
    let update_tag = InstanceUpdateTag {
        weight: match &param.weight {
            Some(v) => *v != 1.0f32,
            None => false,
        },
        metadata: match &param.metadata {
            Some(v) => !v.is_empty() && v != "{}",
            None => false,
        },
        enabled: param.enabled.is_some(),
        ephemeral: param.ephemeral.is_some(),
        from_update: true,
    };
    let instance = param.convert_to_instance();
    match instance {
        Ok(instance) => {
            if !instance.check_valid() {
                HttpResponse::Ok()
                    .json(ApiResult::error(30000, "instance check is invalid".into(), ()))
            } else {
                match appdata
                    .naming_route
                    .update_instance(instance, Some(update_tag))
                    .await
                {
                    Ok(_) => HttpResponse::Ok().json(ApiResult::success("ok".to_string())),
                    Err(e) => {
                        HttpResponse::Ok().json(ApiResult::error(30000, e.to_string(), ()))
                    }
                }
            }
        }
        Err(e) => {
            HttpResponse::Ok().json(ApiResult::error(30000, e, ()))
        }
    }
}

pub async fn del_instance(
    param: web::Query<InstanceWebParams>,
    payload: web::Payload,
    appdata: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    let param = merge_web_param!(param.0, payload);
    let instance = param.convert_to_instance();
    match instance {
        Ok(instance) => {
            if !instance.check_valid() {
                HttpResponse::Ok()
                    .json(ApiResult::error(30000, "instance check is invalid".into(), ()))
            } else {
                match appdata.naming_route.delete_instance(instance).await {
                    Ok(_) => HttpResponse::Ok().json(ApiResult::success("ok".to_string())),
                    Err(e) => {
                        HttpResponse::Ok().json(ApiResult::error(30000, e.to_string(), ()))
                    }
                }
            }
        }
        Err(e) => {
            HttpResponse::Ok().json(ApiResult::error(30000, e, ()))
        }
    }
}

pub async fn get_instance_list(
    param: web::Query<InstanceWebQueryListParams>,
    naming_addr: web::Data<Addr<NamingActor>>,
) -> impl Responder {
    let only_healthy = get_bool_from_string(&param.healthy_only, true);
    let addr = param.get_addr();
    match param.to_clusters_key() {
        Ok((key, clusters)) => {
            match naming_addr
                .send(NamingCmd::QueryListString(
                    key.clone(),
                    clusters,
                    only_healthy,
                    addr,
                ))
                .await
            {
                Ok(res) => {
                    let result: NamingResult = res.unwrap();
                    match result {
                        NamingResult::InstanceListString(v) => {
                            // v is already a JSON string from v1, parse it to wrap in ApiResult
                            let data: serde_json::Value =
                                serde_json::from_str(&v).unwrap_or_default();
                            HttpResponse::Ok()
                                .insert_header(header::ContentType(mime::APPLICATION_JSON))
                                .json(ApiResult::success(data))
                        }
                        _ => HttpResponse::Ok().json(ApiResult::<()>::server_error(())),
                    }
                }
                Err(e) => {
                    HttpResponse::Ok().json(ApiResult::error(30000, e.to_string(), ()))
                }
            }
        }
        Err(e) => {
            HttpResponse::Ok().json(ApiResult::error(30000, e, ()))
        }
    }
}

// ===== Service APIs =====

pub async fn query_service(
    param: web::Query<ServiceQueryListRequest>,
    naming_addr: web::Data<Addr<NamingActor>>,
) -> impl Responder {
    if let Some((group, service_name)) =
        NamingUtils::split_group_and_service_name(&param.0.service_name.clone().unwrap_or_default())
    {
        let namespace_id = NamingUtils::default_namespace(
            param
                .namespace_id
                .as_ref()
                .unwrap_or(&"".to_owned())
                .to_owned(),
        );
        let service_key = ServiceKey::new(&namespace_id, &group, &service_name);
        match naming_addr
            .send(NamingCmd::QueryServiceOnly(service_key.clone()))
            .await
        {
            Ok(res) => {
                let result: NamingResult = res.unwrap();
                match result {
                    NamingResult::ServiceDto(service) => {
                        if let Some(service_dto) = service {
                            let vo = ServiceInfoVo::from_dto(service_dto, service_key.namespace_id);
                            HttpResponse::Ok().json(ApiResult::success(vo))
                        } else {
                            HttpResponse::Ok().json(ApiResult::error(
                                30000,
                                format!(
                                    "service not found,namespace_id:{},service_name:{}",
                                    &namespace_id, &service_name
                                ),
                                (),
                            ))
                        }
                    }
                    _ => HttpResponse::Ok().json(ApiResult::<()>::server_error(())),
                }
            }
            Err(e) => {
                HttpResponse::Ok().json(ApiResult::error(30000, e.to_string(), ()))
            }
        }
    } else {
        HttpResponse::Ok().json(ApiResult::error(
            30000,
            "error,service_name is valid".into(),
            (),
        ))
    }
}

pub async fn update_service(
    param: web::Query<ServiceInfoParam>,
    payload: web::Payload,
    naming_addr: web::Data<Addr<NamingActor>>,
) -> impl Responder {
    let param = merge_web_param!(param.0, payload);
    match param.build_service_info() {
        Ok(service_info) => {
            let _ = naming_addr
                .send(NamingCmd::UpdateService(service_info))
                .await;
            HttpResponse::Ok().json(ApiResult::success("ok".to_string()))
        }
        Err(e) => {
            HttpResponse::Ok().json(ApiResult::error(30000, e.to_string(), ()))
        }
    }
}

pub async fn remove_service(
    param: web::Query<ServiceInfoParam>,
    payload: web::Payload,
    naming_addr: web::Data<Addr<NamingActor>>,
) -> impl Responder {
    let param = merge_web_param!(param.0, payload);
    match param.build_service_info() {
        Ok(service_info) => {
            let key = service_info.to_service_key();
            match naming_addr.send(NamingCmd::RemoveService(key)).await {
                Ok(res) => {
                    let res: anyhow::Result<NamingResult> = res;
                    match res {
                        Ok(_) => {
                            HttpResponse::Ok().json(ApiResult::success("ok".to_string()))
                        }
                        Err(e) => HttpResponse::Ok()
                            .json(ApiResult::error(30000, e.to_string(), ())),
                    }
                }
                Err(e) => {
                    HttpResponse::Ok().json(ApiResult::error(30000, e.to_string(), ()))
                }
            }
        }
        Err(e) => {
            HttpResponse::Ok().json(ApiResult::error(30000, e.to_string(), ()))
        }
    }
}

pub async fn query_service_list(
    param: web::Query<ServiceQueryListRequest>,
    naming_addr: web::Data<Addr<NamingActor>>,
) -> impl Responder {
    let page_size = param.page_size.unwrap_or(0x7fffffff);
    let page_index = param.page_no.unwrap_or(1);
    let namespace_id = NamingUtils::default_namespace(
        param
            .namespace_id
            .as_ref()
            .unwrap_or(&"".to_owned())
            .to_owned(),
    );
    let group = NamingUtils::default_group(
        param
            .group_name
            .as_ref()
            .unwrap_or(&"".to_owned())
            .to_owned(),
    );
    let key = ServiceKey::new(&namespace_id, &group, "");
    match naming_addr
        .send(NamingCmd::QueryServicePage(key, page_size, page_index))
        .await
    {
        Ok(res) => {
            let result: NamingResult = res.unwrap();
            match result {
                NamingResult::ServicePage((c, v)) => {
                    let resp = crate::openapi::naming::model::ServiceQueryListResponce {
                        count: c,
                        doms: v,
                    };
                    HttpResponse::Ok().json(ApiResult::success(resp))
                }
                _ => HttpResponse::Ok().json(ApiResult::<()>::server_error(())),
            }
        }
        Err(e) => {
            HttpResponse::Ok().json(ApiResult::error(30000, e.to_string(), ()))
        }
    }
}
