use crate::openapi::constant::{EMPTY, NAMING_V1_BASE_PATH};
use crate::openapi::naming::catalog::query_opt_service_list;
use crate::openapi::naming::instance::{
    beat_instance, del_instance, get_instance, get_instance_list, update_instance,
};
use crate::openapi::naming::operator::{
    mock_get_switches, mock_operator_metrics, mock_put_switches,
};
use crate::openapi::naming::service::{
    query_service, query_service_list, query_subscribers_list, remove_service, update_service,
};
use crate::openapi::RouteConf;
use actix_web::web::{scope, ServiceConfig};
use actix_web::{web, Scope};

mod catalog;
pub(crate) mod instance;
pub mod model;
mod operator;
pub(crate) mod service;
mod v2;

pub fn openapi_service(conf: RouteConf) -> Vec<Scope> {
    vec![openapi_v1_route(conf)]
}

pub fn openapi_v1_route(_conf: RouteConf) -> Scope {
    web::scope(NAMING_V1_BASE_PATH)
        .service(instance::service())
        .service(service::service())
        .service(operator::service())
        .service(catalog::service())
}

pub fn naming_v1_route(config: &mut ServiceConfig) {
    config
        .service(
            scope("/nacos/v1/ns/instance")
                .service(
                    web::resource(EMPTY)
                        .route(web::get().to(get_instance))
                        .route(web::post().to(update_instance))
                        .route(web::put().to(update_instance))
                        .route(web::patch().to(update_instance))
                        .route(web::delete().to(del_instance)),
                )
                .service(beat_instance)
                .service(get_instance_list),
        )
        .service(
            scope("/nacos/v1/ns/service")
                .service(
                    web::resource(EMPTY)
                        .route(web::post().to(update_service))
                        .route(web::put().to(update_service))
                        .route(web::delete().to(remove_service))
                        .route(web::get().to(query_service)),
                )
                .service(web::resource("/list").route(web::get().to(query_service_list)))
                .service(
                    web::resource("/subscribers").route(web::get().to(query_subscribers_list)),
                ),
        )
        .service(
            scope("/nacos/v1/ns/operator")
                .service(
                    web::resource("switches")
                        .route(web::get().to(mock_get_switches))
                        .route(web::put().to(mock_put_switches)),
                )
                .service(mock_operator_metrics),
        )
        .service(scope("/nacos/v1/ns/catalog").service(query_opt_service_list));
}
