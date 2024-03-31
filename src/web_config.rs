use actix_web::{web, HttpResponse, Responder};

use crate::config::api::app_config as cs_config;

use crate::naming::api::app_config as ns_config;

use crate::console::api::{console_api_config, console_api_config_new};

use crate::auth::mock_token;
use crate::raft::network::raft_config;

use mime_guess::from_path;
use rnacos_web_dist_wrap::get_embedded_file;

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

async fn index() -> impl Responder {
    handle_embedded_file("index.html")
}

#[actix_web::get("/server.svg")]
async fn icon() -> impl Responder {
    handle_embedded_file("server.svg")
}

#[actix_web::get("/rnacos/server.svg")]
async fn console_icon() -> impl Responder {
    handle_embedded_file("rnacos/server.svg")
}

#[actix_web::get("/assets/{_:.*}")]
async fn assets(path: web::Path<String>) -> impl Responder {
    let file = format!("assets/{}", path.as_ref());
    handle_embedded_file(&file)
}

#[actix_web::get("/rnacos/assets/{_:.*}")]
async fn console_assets(path: web::Path<String>) -> impl Responder {
    let file = format!("rnacos/assets/{}", path.as_ref());
    handle_embedded_file(&file)
}

///
/// 面向SDK的http服务接口
pub fn app_config(config: &mut web::ServiceConfig) {
    config.service(web::resource("/nacos/v1/auth/login").route(web::post().to(mock_token)));
    config.service(web::resource("/nacos/v1/auth/users/login").route(web::post().to(mock_token)));
    cs_config(config);
    ns_config(config);
    raft_config(config);
    console_api_config(config);
    console_api_config_new(config);
    console_page_config(config);
}

/// 独立控制台服务
pub fn console_config(config: &mut web::ServiceConfig) {
    cs_config(config);
    ns_config(config);
    console_api_config(config);
    console_api_config_new(config);
    console_page_config(config);
}

pub fn console_page_config(config: &mut web::ServiceConfig) {
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
