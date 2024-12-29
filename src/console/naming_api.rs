#![allow(unused_imports)]

use actix_web::{http::header, web, HttpMessage, HttpRequest, HttpResponse, Responder};

use actix::prelude::Addr;

use super::model::{
    naming_model::{OpsNamingQueryListResponse, QueryAllInstanceListParam},
    PageResult,
};
use crate::naming::core::{NamingActor, NamingCmd, NamingResult};
use crate::user_namespace_privilege;

pub async fn query_ops_instances_list(
    req: HttpRequest,
    param: web::Query<QueryAllInstanceListParam>,
    naming_addr: web::Data<Addr<NamingActor>>,
) -> impl Responder {
    match param.0.to_service_key() {
        Ok(key) => {
            let namespace_privilege = user_namespace_privilege!(req);
            if !namespace_privilege.check_permission(&key.namespace_id) {
                return HttpResponse::Unauthorized().body(format!(
                    "user no such namespace permission: {}",
                    &key.namespace_id
                ));
            }
            match naming_addr.send(NamingCmd::QueryAllInstanceList(key)).await {
                Ok(res) => match res as anyhow::Result<NamingResult> {
                    Ok(result) => match result {
                        NamingResult::InstanceList(list) => {
                            let resp = OpsNamingQueryListResponse {
                                count: list.len() as u64,
                                list,
                            };
                            let v = serde_json::to_string(&resp).unwrap();
                            HttpResponse::Ok()
                                .insert_header(header::ContentType(mime::APPLICATION_JSON))
                                .body(v)
                        }
                        _ => HttpResponse::InternalServerError().body("error result"),
                    },
                    Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
                },
                Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
            }
        }
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

pub async fn query_grpc_client_instance_count(
    naming_addr: web::Data<Addr<NamingActor>>,
) -> impl Responder {
    match naming_addr.send(NamingCmd::QueryClientInstanceCount).await {
        Ok(res) => match res as anyhow::Result<NamingResult> {
            Ok(result) => match result {
                NamingResult::ClientInstanceCount(list) => {
                    let resp = PageResult {
                        count: list.len() as u64,
                        list,
                    };
                    let v = serde_json::to_string(&resp).unwrap();
                    HttpResponse::Ok()
                        .insert_header(header::ContentType(mime::APPLICATION_JSON))
                        .body(v)
                }
                _ => HttpResponse::InternalServerError().body("error result"),
            },
            Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
        },
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}
