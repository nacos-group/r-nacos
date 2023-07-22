use actix_web::{web, HttpResponse, Responder};

use crate::config::api::app_config as cs_config;

use crate::naming::api::app_config as ns_config;

use crate::console::api::app_config as console_config;

use crate::auth::mock_token;
use crate::raft::asyncraft::network::raft_config;

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

#[actix_web::get("/assets/{_:.*}")]
async fn assets(path: web::Path<String>) -> impl Responder {
    let file = format!("assets/{}", path.as_ref());
    handle_embedded_file(&file)
}

pub fn app_config(config: &mut web::ServiceConfig) {
    config.service(web::resource("/nacos/v1/auth/login").route(web::post().to(mock_token)));
    cs_config(config);
    ns_config(config);
    raft_config(config);
    console_config(config);
    console_web_config(config);
}

pub fn console_web_config(config: &mut web::ServiceConfig) {
    config
        .service(web::resource("/").route(web::get().to(index)))
        .service(icon)
        .service(assets)
        .service(web::resource("/index.html").route(web::get().to(index)))
        .service(web::resource("/404").route(web::get().to(index)))
        .service(web::resource("/manage/{_:.*}").route(web::get().to(index)))
        .service(web::resource("/p/{_:.*}").route(web::get().to(index)));
}
