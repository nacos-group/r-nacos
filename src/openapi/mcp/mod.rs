use actix_web::web;

pub mod api;
pub mod model;

pub fn mcp_config(config: &mut web::ServiceConfig) {
    config.service(
        web::resource("/rnacos/mcp/mcp/{id}/{key}").route(web::post().to(api::mcp_handler)),
    );
    config.service(
        web::resource("/rnacos/mcp/mcp/{id}/{key}").route(web::get().to(api::mcp_get_handler)),
    );
    config.service(
        web::resource("/rnacos/mcp/mcp/{id}/{key}")
            .route(web::delete().to(api::mcp_delete_handler)),
    );
    config.service(
        web::resource("/rnacos/mcp/mcp/{id}/{key}/").route(web::post().to(api::mcp_handler)),
    );
    config.service(
        web::resource("/rnacos/mcp/mcp/{id}/{key}/").route(web::get().to(api::mcp_get_handler)),
    );
    config.service(
        web::resource("/rnacos/mcp/mcp/{id}/{key}/")
            .route(web::delete().to(api::mcp_delete_handler)),
    );
}
