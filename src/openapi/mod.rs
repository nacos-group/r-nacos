use actix_web::Responder;
use actix_web::web::{scope, ServiceConfig};

use crate::common::AppSysConfig;
use crate::openapi::constant::{CONFIG_V1_BASE_PATH, NACOS_PREFIX, SLASH};

mod v1;
mod constant;
mod v2;
pub(crate) mod config;
pub(crate) mod naming;

/// r-nacos openapi packages


#[derive(Debug, Default, Clone)]
pub(crate) struct RouteConf {
    pub enable_auth: bool,
    pub auth_key: Option<String>,
}

impl Into<RouteConf> for AppSysConfig
{
    fn into(self) -> RouteConf {
        RouteConf {
            enable_auth: !self.enable_no_auth_console.clone(),
            auth_key: None,
        }
    }
}


/// openapi restful api definition
///
pub fn openapi_config<T>(conf: T) -> impl FnOnce(&mut ServiceConfig)
    where T: Into<RouteConf>
{
    let conf = conf.into();
    move |config: &mut ServiceConfig| {
        let server = scope(NACOS_PREFIX);
        let scope = if conf.enable_auth {
            server
                // .guard(FilterGuard::new())
                .configure(openapi_service(conf))
            // .wrap(AuthVerifyMiddleware::new(conf.auth_key.clone()))
        } else {
            server.configure(openapi_service(conf))
        };
        config.service(scope);
    }
}

fn openapi_service(conf: RouteConf) -> impl FnOnce(&mut ServiceConfig)
{
    move |config: &mut ServiceConfig| {
        config
            // .service(V1_BASE_PATH, v1::openapi_service(conf))
            // .service(V2_BASE_PATH, v2::openapi_service(conf))
            .service(config::openapi_service(conf.clone()))
            .service(naming::openapi_service(conf.clone()));
    }
}
