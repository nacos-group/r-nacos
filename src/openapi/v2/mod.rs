// use actix_web::dev::HttpServiceFactory;
// use actix_web::web;
//
// use crate::openapi::RouteConf;
// use crate::openapi::constant::V2_BASE_PATH;
//
//
// pub fn openapi_service<F>(conf: &RouteConf) -> F where F: HttpServiceFactory + 'static {
//     web::scope(V2_BASE_PATH).service(openapi_route(conf: &RouteConf))
// }
//
// pub fn openapi_route<F>(conf: &RouteConf) -> F where F: HttpServiceFactory + 'static {
//
// }

pub mod model;
pub mod console;