use actix_web::{Scope, web};
use actix_web::dev::HttpServiceFactory;

use crate::openapi::constant::{CONFIG_V1_BASE_PATH, SLASH};
use crate::openapi::RouteConf;

pub mod v2;
pub mod config;

/// current implement for version 1
pub fn openapi_service(conf: RouteConf) -> Scope {
    web::scope(SLASH)
        .service(openapi_route(conf))

}

pub fn openapi_route(conf: RouteConf) -> Scope {
    web::scope(CONFIG_V1_BASE_PATH).service(config::service())
}