use actix_web::{Scope, web};
use actix_web::dev::HttpServiceFactory;

use crate::openapi::constant::{NAMING_V1_BASE_PATH, SLASH};
use crate::openapi::RouteConf;

mod v2;
pub(crate) mod instance;
pub(crate) mod service;
mod operator;
mod catalog;

pub fn openapi_service(conf: RouteConf) -> Scope {
    web::scope(SLASH)
        .service(openapi_route(conf))
}

pub fn openapi_route(conf: RouteConf) -> Scope {
    web::scope(NAMING_V1_BASE_PATH)
        .service(instance::service())
        .service(service::service())
        .service(operator::service())
        .service(catalog::service())
}