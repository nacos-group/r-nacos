use actix_web::web;

pub(crate) mod api;
pub(crate) mod model;

pub fn mcp_route_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/rnacos/v1/mcp")
            .service(web::resource("/server/list").route(web::get().to(api::query_mcp_server_list)))
            .service(
                web::resource("/toolspec/list").route(web::get().to(api::query_tool_spec_list)),
            ),
    );
}
