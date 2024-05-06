use actix_web::{web, Scope};

use crate::openapi::constant::{CONFIG_V1_BASE_PATH, SLASH};
use crate::openapi::RouteConf;

pub mod api;
pub mod v2;

/// current implement for version 1
pub fn openapi_service(conf: RouteConf) -> Scope {
    web::scope(SLASH).service(openapi_route(conf))
}

pub fn openapi_route(_conf: RouteConf) -> Scope {
    web::scope(CONFIG_V1_BASE_PATH).service(api::service())
}
