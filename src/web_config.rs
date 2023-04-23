use actix_web::{Responder, web};

use crate::config::api::{
    app_config as cs_config
};

use crate::naming::api::{
    app_config as ns_config
};

use crate::console::api::{
    app_config as console_config
};

use crate::auth::mock_token;



async fn index() -> impl Responder {
    "nacos server"
}

pub fn app_config(config:&mut web::ServiceConfig){
    config.service(
        web::resource("/").route(web::get().to(index))
    )
    .service(
        web::resource("/nacos/v1/auth/login").route(web::post().to(mock_token))
    )
    ;
    cs_config(config);
    ns_config(config);
    console_config(config);
}