use actix::prelude::*;
use actix_web::{http::header, web, HttpResponse, Responder};

use crate::config::core::ConfigActor;
//use crate::console::raft_api::{raft_add_learner, raft_change_membership, raft_init, raft_metrics, raft_read, raft_write};

use super::config_api::query_config_list;
use super::{
    config_api::{download_config, import_config, query_history_config_page},
    connection_api::query_grpc_connection,
    model::{ConsoleResult, NamespaceInfo},
    naming_api::{query_grpc_client_instance_count, query_ops_instances_list},
    NamespaceUtils,
};

pub async fn query_namespace_list(config_addr: web::Data<Addr<ConfigActor>>) -> impl Responder {
    //HttpResponse::InternalServerError().body("system error")
    let namespaces = NamespaceUtils::get_namespaces(&config_addr).await;
    let result = ConsoleResult::success(namespaces);
    let v = serde_json::to_string(&result).unwrap();
    HttpResponse::Ok()
        .insert_header(header::ContentType(mime::APPLICATION_JSON))
        .body(v)
}

pub async fn add_namespace(
    param: web::Form<NamespaceInfo>,
    config_addr: web::Data<Addr<ConfigActor>>,
) -> impl Responder {
    match NamespaceUtils::add_namespace(&config_addr, param.0).await {
        Ok(_) => {
            let result = ConsoleResult::success(true);
            let v = serde_json::to_string(&result).unwrap();
            HttpResponse::Ok()
                .insert_header(header::ContentType(mime::APPLICATION_JSON))
                .body(v)
        }
        Err(e) => {
            let result: ConsoleResult<()> = ConsoleResult::error(e.to_string());
            let v = serde_json::to_string(&result).unwrap();
            HttpResponse::Ok()
                .insert_header(header::ContentType(mime::APPLICATION_JSON))
                .body(v)
        }
    }
}

pub async fn update_namespace(
    param: web::Form<NamespaceInfo>,
    config_addr: web::Data<Addr<ConfigActor>>,
) -> impl Responder {
    match NamespaceUtils::update_namespace(&config_addr, param.0).await {
        Ok(_) => {
            let result = ConsoleResult::success(true);
            let v = serde_json::to_string(&result).unwrap();
            HttpResponse::Ok()
                .insert_header(header::ContentType(mime::APPLICATION_JSON))
                .body(v)
        }
        Err(e) => {
            let result: ConsoleResult<()> = ConsoleResult::error(e.to_string());
            let v = serde_json::to_string(&result).unwrap();
            HttpResponse::Ok()
                .insert_header(header::ContentType(mime::APPLICATION_JSON))
                .body(v)
        }
    }
}

pub async fn remove_namespace(
    param: web::Form<NamespaceInfo>,
    config_addr: web::Data<Addr<ConfigActor>>,
) -> impl Responder {
    match NamespaceUtils::remove_namespace(&config_addr, param.0.namespace_id).await {
        Ok(_) => {
            let result = ConsoleResult::success(true);
            let v = serde_json::to_string(&result).unwrap();
            HttpResponse::Ok()
                .insert_header(header::ContentType(mime::APPLICATION_JSON))
                .body(v)
        }
        Err(e) => {
            let result: ConsoleResult<()> = ConsoleResult::error(e.to_string());
            let v = serde_json::to_string(&result).unwrap();
            HttpResponse::Ok()
                .insert_header(header::ContentType(mime::APPLICATION_JSON))
                .body(v)
        }
    }
}

pub fn app_config(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/nacos/v1/console")
            .service(
                web::resource("/namespaces")
                    .route(web::get().to(query_namespace_list))
                    .route(web::post().to(add_namespace))
                    .route(web::put().to(update_namespace))
                    .route(web::delete().to(remove_namespace)),
            )
            .service(web::resource("/configs").route(web::get().to(query_config_list)))
            .service(web::resource("/config/import").route(web::post().to(import_config)))
            .service(web::resource("/config/download").route(web::get().to(download_config)))
            .service(
                web::resource("/config/history").route(web::get().to(query_history_config_page)),
            )
            .service(web::resource("/instances").route(web::get().to(query_ops_instances_list)))
            .service(
                web::resource("/naming/client_instance_count")
                    .route(web::get().to(query_grpc_client_instance_count)),
            )
            /* 
            .service( web::resource("/raft/init") .route(web::post().to(raft_init)))
            .service( web::resource("/raft/add_learner") .route(web::post().to(raft_add_learner)))
            .service( web::resource("/raft/change_membership") .route(web::post().to(raft_change_membership)))
            .service( web::resource("/raft/metrics") .route(web::get().to(raft_metrics)))
            .service( web::resource("/raft/write") .route(web::post().to(raft_write)))
            .service( web::resource("/raft/read") .route(web::post().to(raft_read)))
            */
            .service(web::resource("/connections").route(web::get().to(query_grpc_connection))),
    );
}
