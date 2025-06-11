#![allow(unused_imports, unused_assignments, unused_variables)]
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use actix::prelude::*;
use actix_web::dev::HttpServiceFactory;
use actix_web::{get, http::header, put, web, HttpResponse, Responder, Scope};
use serde::{Deserialize, Serialize};

use crate::common::appdata::AppShareData;
use crate::common::web_utils::get_req_body;
use crate::merge_web_param;
use crate::naming::api_model::InstanceVO;
use crate::naming::core::{NamingActor, NamingCmd, NamingResult};
use crate::naming::model::{Instance, InstanceUpdateTag, ServiceKey};
use crate::naming::{
    NamingUtils, CLIENT_BEAT_INTERVAL_KEY, LIGHT_BEAT_ENABLED_KEY, RESPONSE_CODE_KEY,
    RESPONSE_CODE_OK,
};
use crate::openapi::constant::EMPTY;
use crate::openapi::naming::model::{BeatRequest, InstanceWebParams, InstanceWebQueryListParams};
use crate::utils::{get_bool_from_string, select_option_by_clone};

pub(super) fn service() -> Scope {
    web::scope("/instance")
        .service(
            web::resource(EMPTY)
                .route(web::get().to(get_instance))
                .route(web::post().to(update_instance))
                .route(web::put().to(update_instance))
                .route(web::patch().to(update_instance))
                .route(web::delete().to(del_instance)),
        )
        .service(beat_instance)
        .service(get_instance_list)
}

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
                        HttpResponse::Ok()
                            .insert_header(header::ContentType(mime::APPLICATION_JSON))
                            .body(serde_json::to_string(&vo).unwrap())
                    }
                    _ => HttpResponse::InternalServerError().body("error"),
                }
            }
            Err(_) => HttpResponse::InternalServerError().body("error"),
        },
        Err(e) => HttpResponse::InternalServerError().body(e),
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
        enabled: match &param.enabled {
            Some(v) => true,
            None => false,
        },
        ephemeral: match &param.ephemeral {
            Some(v) => true,
            None => false,
        },
        from_update: true,
    };
    let instance = param.convert_to_instance();
    match instance {
        Ok(instance) => {
            if !instance.check_valid() {
                HttpResponse::InternalServerError().body("instance check is invalid")
            } else {
                match appdata
                    .naming_route
                    .update_instance(instance, Some(update_tag))
                    .await
                {
                    Ok(_) => HttpResponse::Ok().body("ok"),
                    Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
                }
            }
        }
        Err(e) => HttpResponse::InternalServerError().body(e),
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
                HttpResponse::InternalServerError().body("instance check is invalid")
            } else {
                match appdata.naming_route.delete_instance(instance).await {
                    Ok(_) => HttpResponse::Ok().body("ok"),
                    Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
                }
            }
        }
        Err(e) => HttpResponse::InternalServerError().body(e),
    }
}

#[put("/beat")]
pub async fn beat_instance(
    param: web::Query<BeatRequest>,
    payload: web::Payload,
    appdata: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    let param = merge_web_param!(param.0, payload);
    //debug
    //log::info!("beat request param:{}",serde_json::to_string(&param).unwrap());
    let instance = param.convert_to_instance();
    match instance {
        Ok(instance) => {
            if !instance.check_valid() {
                HttpResponse::InternalServerError().body("instance check is invalid")
            } else {
                let tag = InstanceUpdateTag {
                    weight: false,
                    enabled: false,
                    ephemeral: false,
                    metadata: false,
                    from_update: false,
                };
                match appdata
                    .naming_route
                    .update_instance(instance, Some(tag))
                    .await
                {
                    Ok(_) => {
                        let mut result = HashMap::new();
                        result.insert(RESPONSE_CODE_KEY, serde_json::json!(RESPONSE_CODE_OK));
                        result.insert(CLIENT_BEAT_INTERVAL_KEY, serde_json::json!(5000));
                        result.insert(LIGHT_BEAT_ENABLED_KEY, serde_json::json!(true));
                        let v = serde_json::to_string(&result).unwrap();
                        HttpResponse::Ok()
                            .insert_header(header::ContentType(mime::APPLICATION_JSON))
                            .body(v)
                    }
                    Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
                }
            }
        }
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[get("/list")]
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
                        NamingResult::InstanceListString(v) => HttpResponse::Ok()
                            .insert_header(header::ContentType(mime::APPLICATION_JSON))
                            .body(v),
                        _ => HttpResponse::InternalServerError().body("error"),
                    }
                }
                Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
            }
        }
        Err(err) => HttpResponse::InternalServerError().body(err),
    }
}
