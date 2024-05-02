use actix_web::{Responder, web};

pub(crate) async fn mock_token() -> impl Responder {
    "{\"accessToken\":\"mock_token\",\"tokenTtl\":18000,\"globalAdmin\":true}"
}

pub fn login_config(config: &mut web::ServiceConfig) {
    config
        .service(web::resource("/nacos/v1/auth/users/login").route(web::post().to(mock_token)))
        .service(web::resource("/nacos/v1/auth/login").route(web::post().to(mock_token)));
}