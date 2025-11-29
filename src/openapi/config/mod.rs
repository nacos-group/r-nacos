use crate::openapi::config::api::{add_config, del_config, get_config, listener_config};
use crate::openapi::constant::{CONFIG_V1_BASE_PATH, EMPTY};
use crate::openapi::RouteConf;
use actix_web::web::{scope, ServiceConfig};
use actix_web::{web, Scope};

pub mod api;
pub mod v2;

/// current implement for version 1
pub fn openapi_service(conf: RouteConf) -> Vec<Scope> {
    vec![openapi_v1_route(conf)]
}

pub fn openapi_v1_route(_conf: RouteConf) -> Scope {
    web::scope(CONFIG_V1_BASE_PATH).service(api::service())
}

pub fn config_v1_route(config: &mut ServiceConfig) {
    config.service(
        scope("/nacos/v1/cs/configs")
            .service(
                web::resource(EMPTY)
                    .route(web::get().to(get_config))
                    .route(web::post().to(add_config))
                    .route(web::put().to(add_config))
                    .route(web::delete().to(del_config)),
            )
            .service(web::resource("/listener").route(web::post().to(listener_config))),
    );
}
