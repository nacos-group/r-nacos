// use actix_web::dev::HttpServiceFactory;
// use actix_web::web;
// use crate::openapi::constant::V1_BASE_PATH;
// use crate::openapi::RouteConf;
//
// pub fn openapi_service<F>(conf: &RouteConf) -> F where F: HttpServiceFactory + 'static {
//     web::scope(V1_BASE_PATH).service(openapi_route(conf: &RouteConf))
// }
//
// pub fn openapi_route<F>(conf: &RouteConf) -> F where F: HttpServiceFactory + 'static {
//
// }

pub mod console;
