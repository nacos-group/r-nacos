use actix_web::web;

pub mod api;
pub mod model;
pub mod sse;

lazy_static::lazy_static! {
    pub(crate) static ref IGNORE_TRASFER_HEADERS: Vec<&'static  str> =  vec!["host","content-type","content-length","accept-encoding","accept","connection"];
}

pub enum HandleOtherResult {
    Accepted,
}

pub fn mcp_config(config: &mut web::ServiceConfig) {
    config.service(
        web::resource("/rnacos/mcp/sse/messages/{node_id}/{server_key}/{session_id}")
            .route(web::post().to(sse::sse_message)),
    );
    config.service(
        web::resource("/rnacos/mcp/sse/messages/{node_id}/{server_key}/{session_id}/")
            .route(web::post().to(sse::sse_message)),
    );
    config.service(
        web::resource("/rnacos/mcp/sse/{server_key}/{auth_key}")
            .route(web::get().to(sse::sse_connect)),
    );
    config.service(
        web::resource("/rnacos/mcp/sse/{server_key}/{auth_key}/")
            .route(web::get().to(sse::sse_connect)),
    );
    config.service(
        web::resource("/rnacos/mcp/{server_key}/{auth_key}")
            .route(web::post().to(api::mcp_handler)),
    );
    config.service(
        web::resource("/rnacos/mcp/{server_key}/{auth_key}")
            .route(web::get().to(api::mcp_get_handler)),
    );
    config.service(
        web::resource("/rnacos/mcp/{server_key}/{auth_key}")
            .route(web::delete().to(api::mcp_delete_handler)),
    );
    config.service(
        web::resource("/rnacos/mcp/{server_key}/{auth_key}/")
            .route(web::post().to(api::mcp_handler)),
    );
    config.service(
        web::resource("/rnacos/mcp/{server_key}/{auth_key}/")
            .route(web::get().to(api::mcp_get_handler)),
    );
    config.service(
        web::resource("/rnacos/mcp/{server_key}/{auth_key}/")
            .route(web::delete().to(api::mcp_delete_handler)),
    );
}
