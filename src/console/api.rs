use std::sync::Arc;

use super::cluster_api::query_cluster_info;
use super::config_api::{download_config_by_keys, query_config_list};
use super::{
    config_api::{download_config, import_config, query_history_config_page},
    connection_api::query_grpc_connection,
    model::{ConsoleResult, NamespaceInfo},
    naming_api::{query_grpc_client_instance_count, query_ops_instances_list},
    transfer_api, NamespaceUtils,
};
use super::{login_api, user_api};
use crate::common::appdata::AppShareData;
use crate::common::error_code::NO_PERMISSION;
use crate::common::string_utils::StringUtils;
use crate::naming::ops::ops_api::query_opt_service_list;
use crate::openapi::naming::instance::{del_instance, get_instance, update_instance};
use crate::openapi::naming::service::{
    query_service, query_subscribers_list, remove_service, update_service,
};
use crate::user_namespace_privilege;
use actix_web::{http::header, web, HttpMessage, HttpRequest, HttpResponse, Responder};
use uuid::Uuid;

use super::v2;

pub async fn query_namespace_list(
    req: HttpRequest,
    app_data: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    let namespace_privilege = user_namespace_privilege!(req);
    //HttpResponse::InternalServerError().body("system error")
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
    let result = ConsoleResult::success(namespaces);
    let v = serde_json::to_string(&result).unwrap();
    HttpResponse::Ok()
        .insert_header(header::ContentType(mime::APPLICATION_JSON))
        .body(v)
}

pub async fn add_namespace(
    req: HttpRequest,
    param: web::Form<NamespaceInfo>,
    app_data: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    let mut param = param.0;
    if StringUtils::is_option_empty_arc(&param.namespace_id) {
        param.namespace_id = Some(Arc::new(Uuid::new_v4().to_string()));
    }
    let namespace_privilege = user_namespace_privilege!(req);
    if !namespace_privilege.check_option_value_permission(&param.namespace_id, false) {
        return HttpResponse::Ok().json(ConsoleResult::<()>::error(NO_PERMISSION.to_string()));
    }
    match NamespaceUtils::add_namespace(&app_data, param).await {
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
    req: HttpRequest,
    param: web::Form<NamespaceInfo>,
    app_data: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    let namespace_privilege = user_namespace_privilege!(req);
    if !namespace_privilege.check_option_value_permission(&param.namespace_id, false) {
        return HttpResponse::Ok().json(ConsoleResult::<()>::error(NO_PERMISSION.to_string()));
    }
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
    req: HttpRequest,
    param: web::Form<NamespaceInfo>,
    app_data: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    let namespace_privilege = user_namespace_privilege!(req);
    if !namespace_privilege.check_option_value_permission(&param.namespace_id, false) {
        return HttpResponse::Ok().json(ConsoleResult::<()>::error(NO_PERMISSION.to_string()));
    }
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

pub fn console_api_config_v1(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/rnacos/api/console")
            .service(
                web::resource("/cs/configs")
                    .route(web::get().to(crate::openapi::config::api::get_config))
                    .route(web::post().to(crate::openapi::config::api::add_config))
                    .route(web::put().to(crate::openapi::config::api::add_config))
                    .route(web::delete().to(crate::openapi::config::api::del_config)),
            )
            .service(
                web::resource("/ns/service")
                    .route(web::post().to(update_service))
                    .route(web::put().to(update_service))
                    .route(web::delete().to(remove_service))
                    .route(web::get().to(query_service)),
            )
            .service(
                web::resource("/ns/service/subscribers")
                    .route(web::get().to(query_subscribers_list)),
            )
            .service(
                web::resource("/ns/instance")
                    .route(web::get().to(get_instance))
                    .route(web::post().to(update_instance))
                    .route(web::put().to(update_instance))
                    .route(web::delete().to(del_instance)),
            )
            .service(web::resource("/ns/services").route(web::get().to(query_opt_service_list)))
            .service(
                web::resource("/namespaces")
                    .route(web::get().to(query_namespace_list))
                    .route(web::post().to(add_namespace))
                    .route(web::put().to(update_namespace))
                    .route(web::delete().to(remove_namespace)),
            )
            .service(web::resource("/configs").route(web::get().to(query_config_list)))
            .service(web::resource("/config/import").route(web::post().to(import_config)))
            .service(
                web::resource("/config/download")
                    .route(web::get().to(download_config))
                    .route(web::post().to(download_config_by_keys)),
            )
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
                web::resource("/user/web_resources")
                    .route(web::get().to(user_api::get_user_web_resources)),
            )
            .service(web::resource("/user/list").route(web::get().to(user_api::get_user_page_list)))
            .service(web::resource("/user/add").route(web::post().to(user_api::add_user)))
            .service(web::resource("/user/update").route(web::post().to(user_api::update_user)))
            .service(web::resource("/user/remove").route(web::post().to(user_api::remove_user)))
            .service(
                web::resource("/user/reset_password")
                    .route(web::post().to(user_api::reset_password)),
            )
            .service(
                web::resource("/transfer/export")
                    .route(web::get().to(transfer_api::download_transfer_file)),
            )
            .service(
                web::resource("/transfer/import")
                    .route(web::post().to(transfer_api::import_transfer_file)),
            ),
    );
}

pub fn console_api_config_v2(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/rnacos/api/console/v2")
            .service(web::resource("/login/login").route(web::post().to(v2::login_api::login)))
            .service(
                web::resource("/login/captcha").route(web::get().to(v2::login_api::gen_captcha)),
            )
            .service(web::resource("/login/logout").route(web::post().to(v2::login_api::logout)))
            .service(web::resource("/user/info").route(web::get().to(v2::user_api::get_user_info)))
            .service(
                web::resource("/user/list").route(web::get().to(v2::user_api::get_user_page_list)),
            )
            .service(web::resource("/user/add").route(web::post().to(v2::user_api::add_user)))
            .service(web::resource("/user/update").route(web::post().to(v2::user_api::update_user)))
            .service(web::resource("/user/remove").route(web::post().to(v2::user_api::remove_user)))
            .service(
                web::resource("/user/web_resources")
                    .route(web::get().to(v2::user_api::get_user_web_resources)),
            )
            .service(
                web::resource("/user/reset_password")
                    .route(web::post().to(v2::user_api::reset_password)),
            )
            .service(
                web::resource("/namespaces/list")
                    .route(web::get().to(v2::namespace_api::query_namespace_list)),
            )
            .service(
                web::resource("/namespaces/add")
                    .route(web::post().to(v2::namespace_api::add_namespace)),
            )
            .service(
                web::resource("/namespaces/update")
                    .route(web::post().to(v2::namespace_api::update_namespace)),
            )
            .service(
                web::resource("/namespaces/remove")
                    .route(web::post().to(v2::namespace_api::remove_namespace)),
            )
            .service(
                web::resource("/cluster/cluster_node_list")
                    .route(web::get().to(v2::cluster_api::query_cluster_info)),
            )
            .service(
                web::resource("/config/import")
                    .route(web::post().to(v2::config_api::import_config)),
            )
            .service(
                web::resource("/config/download")
                    .route(web::get().to(v2::config_api::download_config)),
            )
            .service(
                web::resource("/config/list")
                    .route(web::get().to(v2::config_api::query_config_list)),
            )
            .service(web::resource("/config/info").route(web::get().to(v2::config_api::get_config)))
            .service(web::resource("/config/add").route(web::post().to(v2::config_api::add_config)))
            .service(
                web::resource("/config/update").route(web::post().to(v2::config_api::add_config)),
            )
            .service(
                web::resource("/config/remove")
                    .route(web::post().to(v2::config_api::remove_config)),
            )
            .service(
                web::resource("/config/history")
                    .route(web::get().to(v2::config_api::query_history_config_page)),
            )
            .service(
                web::resource("/service/list")
                    .route(web::get().to(v2::naming_api::query_service_list)),
            )
            .service(
                web::resource("/service/subscriber/list")
                    .route(web::get().to(v2::naming_api::query_subscribers_list)),
            )
            .service(
                web::resource("/service/add").route(web::post().to(v2::naming_api::add_service)),
            )
            .service(
                web::resource("/service/update").route(web::post().to(v2::naming_api::add_service)),
            )
            .service(
                web::resource("/service/remove")
                    .route(web::post().to(v2::naming_api::remove_service)),
            )
            .service(
                web::resource("/instance/list")
                    .route(web::get().to(v2::naming_api::query_instances_list)),
            )
            .service(
                web::resource("/instance/info").route(web::get().to(v2::naming_api::get_instance)),
            )
            .service(
                web::resource("/instance/add").route(web::post().to(v2::naming_api::add_instance)),
            )
            .service(
                web::resource("/instance/update")
                    .route(web::post().to(v2::naming_api::add_instance)),
            )
            .service(
                web::resource("/instance/remove")
                    .route(web::post().to(v2::naming_api::remove_instance)),
            )
            .service(
                web::resource("/transfer/export")
                    .route(web::get().to(transfer_api::download_transfer_file)),
            )
            .service(
                web::resource("/transfer/import")
                    .route(web::post().to(transfer_api::import_transfer_file)),
            )
            .service(
                web::resource("/metrics/timeline")
                    .route(web::get().to(v2::metrics_api::query_metrics_timeline))
                    .route(web::post().to(v2::metrics_api::query_metrics_timeline_json)),
            )
            .service(
                web::resource("/mcp/toolspec/list")
                    .route(web::get().to(v2::mcp_tool_spec_api::query_tool_spec_list)),
            )
            .service(
                web::resource("/mcp/toolspec/info")
                    .route(web::get().to(v2::mcp_tool_spec_api::get_tool_spec)),
            )
            .service(
                web::resource("/mcp/toolspec/add")
                    .route(web::post().to(v2::mcp_tool_spec_api::add_or_update_tool_spec)),
            )
            .service(
                web::resource("/mcp/toolspec/update")
                    .route(web::post().to(v2::mcp_tool_spec_api::add_or_update_tool_spec)),
            )
            .service(
                web::resource("/mcp/toolspec/remove")
                    .route(web::post().to(v2::mcp_tool_spec_api::remove_tool_spec)),
            )
            .service(
                web::resource("/mcp/toolspec/batch_update")
                    .route(web::post().to(v2::mcp_tool_spec_api::update_tool_specs)),
            )
            .service(
                web::resource("/mcp/toolspec/download")
                    .route(web::get().to(v2::mcp_tool_spec_api::download_tool_specs)),
            )
            .service(
                web::resource("/mcp/toolspec/import")
                    .route(web::post().to(v2::mcp_tool_spec_api::import_tool_specs)),
            )
            // McpServer控制台接口路由
            .service(
                web::resource("/mcp/server/list")
                    .route(web::get().to(v2::mcp_server_api::query_mcp_server_list)),
            )
            .service(
                web::resource("/mcp/server/info")
                    .route(web::get().to(v2::mcp_server_api::get_mcp_server)),
            )
            .service(
                web::resource("/mcp/server/add")
                    .route(web::post().to(v2::mcp_server_api::add_mcp_server)),
            )
            .service(
                web::resource("/mcp/server/update")
                    .route(web::post().to(v2::mcp_server_api::update_mcp_server)),
            )
            .service(
                web::resource("/mcp/server/remove")
                    .route(web::post().to(v2::mcp_server_api::remove_mcp_server)),
            )
            .service(
                web::resource("/mcp/server/history")
                    .route(web::get().to(v2::mcp_server_api::query_mcp_server_history)),
            )
            .service(
                web::resource("/mcp/server/publish")
                    .route(web::post().to(v2::mcp_server_api::publish_current_mcp_server)),
            )
            .service(
                web::resource("/mcp/server/publish/history")
                    .route(web::post().to(v2::mcp_server_api::publish_history_mcp_server)),
            ),
    );
}
