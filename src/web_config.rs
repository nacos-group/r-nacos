use actix_web::web::ServiceConfig;
use actix_web::{web, HttpResponse, Responder};
use mime_guess::from_path;
use rnacos_web_dist_wrap::get_embedded_file;

use crate::common::AppSysConfig;
use crate::console::api::{console_api_config_v1, console_api_config_v2};
use crate::openapi::auth::{login_config, mock_token};
use crate::openapi::backup::backup_config;
#[cfg(feature = "debug")]
use crate::openapi::debug::debug_config;
use crate::openapi::health::health_config;
use crate::openapi::mcp::mcp_config;
use crate::openapi::metrics::metrics_config;
use crate::openapi::{
    openapi_config, v1::console as nacos_console, v2::console as nacos_console_v2,
};
use crate::raft::network::raft_config;

/*
use rust_embed::RustEmbed;
#[derive(RustEmbed)]
#[folder = "target/rnacos-web"]
struct Asset;
*/

fn handle_embedded_file(path: &str) -> HttpResponse {
    match get_embedded_file(path) {
        Some(content) => HttpResponse::Ok()
            .content_type(from_path(path).first_or_octet_stream().as_ref())
            .body(content.data.into_owned()),
        None => HttpResponse::NotFound().body("404 Not Found"),
    }
}

fn handle_embedded_file_with_cache(path: &str) -> HttpResponse {
    match get_embedded_file(path) {
        Some(content) => HttpResponse::Ok()
            .content_type(from_path(path).first_or_octet_stream().as_ref())
            .insert_header(("Cache-Control", "max-age=604800, public"))
            .body(content.data.into_owned()),
        None => HttpResponse::NotFound().body("404 Not Found"),
    }
}

async fn index() -> impl Responder {
    handle_embedded_file("index.html")
}

#[actix_web::get("/server.svg")]
async fn icon() -> impl Responder {
    handle_embedded_file_with_cache("server.svg")
}

#[actix_web::get("/rnacos/server.svg")]
async fn console_icon() -> impl Responder {
    handle_embedded_file_with_cache("rnacos/server.svg")
}

#[actix_web::get("/assets/{_:.*}")]
async fn assets(path: web::Path<String>) -> impl Responder {
    let file = format!("assets/{}", path.as_ref());
    handle_embedded_file_with_cache(&file)
}

#[actix_web::get("/rnacos/assets/{_:.*}")]
async fn console_assets(path: web::Path<String>) -> impl Responder {
    let file = format!("rnacos/assets/{}", path.as_ref());
    handle_embedded_file_with_cache(&file)
}

async fn disable_no_auth_console_index() -> impl Responder {
    let body = "<!DOCTYPE html>
<html lang='en'>
  <head>
    <meta charset='UTF-8' />
    <meta name='viewport' content='width=device-width, initial-scale=1.0' />
    <title>R-NACOS</title>
  </head>
  <body>
    <p>R-NACOS 未开启无鉴权控制台。</p>
    <p>请使用鉴权控制台: http://localhost:10848/rnacos/ </p>
    <p>或者通过配置 RNACOS_ENABLE_NO_AUTH_CONSOLE=true 开启无鉴权控制台。</p>
  </body>
</html>";
    HttpResponse::Ok().content_type("text/html").body(body)
}

///
/// 面向SDK的http服务接口
pub fn app_config(conf_data: AppSysConfig) -> impl FnOnce(&mut ServiceConfig) {
    move |config: &mut ServiceConfig| {
        if !conf_data.enable_no_auth_console || conf_data.openapi_enable_auth {
            backup_config(config);
            mcp_config(config);
            config
                .service(web::resource("/").route(web::get().to(disable_no_auth_console_index)))
                .service(
                    web::resource("/nacos").route(web::get().to(disable_no_auth_console_index)),
                )
                .service(
                    web::resource("/nacos/").route(web::get().to(disable_no_auth_console_index)),
                )
                .service(
                    web::resource("/rnacos").route(web::get().to(disable_no_auth_console_index)),
                )
                .service(
                    web::resource("/rnacos/{_:.*}")
                        .route(web::get().to(disable_no_auth_console_index)),
                );
            login_config(config);
            metrics_config(config);
            health_config(config);
            raft_config(config);
            nacos_console_api_config(config);
            config.configure(openapi_config(conf_data));
            #[cfg(feature = "debug")]
            debug_config(config);
        } else {
            backup_config(config);
            mcp_config(config);
            login_config(config);
            metrics_config(config);
            health_config(config);
            raft_config(config);
            nacos_console_api_config(config);
            config.configure(openapi_config(conf_data));
            console_api_config_v2(config);
            console_api_config_v1(config);
            console_page_config(config);
            #[cfg(feature = "debug")]
            debug_config(config);
        };
    }
}

#[deprecated]
pub fn app_without_no_auth_console_config(config: &mut ServiceConfig) {
    config
        .service(web::resource("/").route(web::get().to(disable_no_auth_console_index)))
        .service(web::resource("/nacos").route(web::get().to(disable_no_auth_console_index)))
        .service(web::resource("/nacos/").route(web::get().to(disable_no_auth_console_index)))
        .service(web::resource("/rnacos").route(web::get().to(disable_no_auth_console_index)))
        .service(
            web::resource("/rnacos/{_:.*}").route(web::get().to(disable_no_auth_console_index)),
        )
        .service(web::resource("/nacos/v1/auth/login").route(web::post().to(mock_token)))
        .service(web::resource("/nacos/v1/auth/users/login").route(web::post().to(mock_token)));
    raft_config(config);
}

pub fn nacos_console_api_config(config: &mut ServiceConfig) {
    config.service(
        web::scope("/nacos/v1/console").service(
            web::resource("/namespaces")
                .route(web::get().to(nacos_console::namespace::query_namespace_list))
                .route(web::post().to(nacos_console::namespace::add_namespace))
                .route(web::put().to(nacos_console::namespace::update_namespace))
                .route(web::delete().to(nacos_console::namespace::remove_namespace)),
        ),
    );

    config.service(
        web::scope("/nacos/v2/console")
            .service(
                web::resource("/namespace/list")
                    .route(web::get().to(nacos_console_v2::namespace::query_namespace_list)),
            )
            .service(
                web::resource("/namespace")
                    .route(web::get().to(nacos_console_v2::namespace::query_namespace))
                    .route(web::post().to(nacos_console_v2::namespace::add_namespace))
                    .route(web::put().to(nacos_console_v2::namespace::update_namespace))
                    .route(web::delete().to(nacos_console_v2::namespace::remove_namespace)),
            ),
    );
}

/// 独立控制台服务
pub fn console_config(config: &mut ServiceConfig) {
    //console_api_config(config);
    console_api_config_v2(config);
    console_api_config_v1(config);
    console_page_config(config);
}

pub fn console_page_config(config: &mut ServiceConfig) {
    config
        .service(web::resource("/").route(web::get().to(index)))
        .service(icon)
        .service(assets)
        .service(web::resource("/index.html").route(web::get().to(index)))
        .service(web::resource("/404").route(web::get().to(index)))
        .service(web::resource("/nopermission").route(web::get().to(index)))
        .service(web::resource("/manage/{_:.*}").route(web::get().to(index)))
        .service(web::resource("/p/{_:.*}").route(web::get().to(index)))
        //new console path
        .service(web::resource("/rnacos").route(web::get().to(index)))
        .service(web::resource("/rnacos/").route(web::get().to(index)))
        .service(console_icon)
        .service(console_assets)
        .service(web::resource("/rnacos/index.html").route(web::get().to(index)))
        .service(web::resource("/rnacos/404").route(web::get().to(index)))
        .service(web::resource("/rnacos/nopermission").route(web::get().to(index)))
        .service(web::resource("/rnacos/manage/{_:.*}").route(web::get().to(index)))
        .service(web::resource("/rnacos/p/{_:.*}").route(web::get().to(index)));
}
