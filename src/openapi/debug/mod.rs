use actix_web::web;

pub(crate) mod naming_api;
pub(crate) mod sequence_api;

pub fn debug_config(config: &mut web::ServiceConfig) {
    config
        .service(
            web::resource("/rnacos/debug/naming/common")
                .route(web::get().to(naming_api::naming_debug_req)),
        )
        .service(
            web::resource("/rnacos/debug/sequence/next_id")
                .route(web::get().to(sequence_api::next_id)),
        );
}
