use std::sync::Arc;

use actix::prelude::*;
use actix_web::{http::header, web, HttpResponse, Responder};

use crate::common::appdata::AppShareData;
use crate::config::core::ConfigActor;
//use crate::console::raft_api::{raft_add_learner, raft_change_membership, raft_init, raft_metrics, raft_read, raft_write};

use super::cluster_api::query_cluster_info;
use super::config_api::query_config_list;
use super::{
    config_api::{download_config, import_config, query_history_config_page},
    connection_api::query_grpc_connection,
    model::{ConsoleResult, NamespaceInfo},
    naming_api::{query_grpc_client_instance_count, query_ops_instances_list},
    NamespaceUtils,
};
use super::{login_api, user_api};

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
    app_data: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    match NamespaceUtils::add_namespace(&app_data, param.0).await {
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
    app_data: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    match NamespaceUtils::update_namespace(&app_data, param.0).await {
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
    app_data: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    match NamespaceUtils::remove_namespace(&app_data, param.0.namespace_id).await {
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
            .service(
                web::resource("/cluster/cluster_node_list")
                    .route(web::get().to(query_cluster_info)),
            )
            .service(web::resource("/connections").route(web::get().to(query_grpc_connection)))
            .service(web::resource("/login/login").route(web::post().to(login_api::login)))
            .service(web::resource("/login/captcha").route(web::get().to(login_api::gen_captcha)))
            .service(web::resource("/login/logout").route(web::post().to(login_api::logout)))
            .service(web::resource("/user/info").route(web::get().to(user_api::get_user_info)))
            .service(
                web::resource("/user/reset_password")
                    .route(web::post().to(user_api::reset_password)),
            ),
    );
}
