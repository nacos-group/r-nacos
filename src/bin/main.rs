
use std::collections::HashMap;
use nacos_rust::naming::listener::InnerNamingListener;
use tokio::net::UdpSocket;
use std::error::Error;
use std::net::SocketAddr;

use nacos_rust::config::config::{
    ConfigActor,ConfigCmd,ConfigKey,ConfigResult,ListenerItem,ListenerResult
};
use nacos_rust::config::api::{
    app_config as cs_config
};

use nacos_rust::naming::core::{NamingActor,NamingCmd};
use nacos_rust::naming::api::{
    app_config as ns_config
};

use actix::prelude::{
    Actor,Addr,
};
use actix_web::{
    App,web,HttpRequest,HttpResponse,Responder,HttpMessage,
    middleware,HttpServer,
};

async fn index() -> impl Responder {
    "hello"
}

async fn mock_token() -> impl Responder {
    "{\"accessToken\":\"mock_value\",\"tokenTtl\":18000,\"globalAdmin\":true}"
}

fn app_config(config:&mut web::ServiceConfig){
    config.service(
        web::resource("/").route(web::get().to(index))
    )
    .service(
        web::resource("/nacos/v1/auth/login").route(web::post().to(mock_token))
    )
    ;
    cs_config(config);
    ns_config(config);
}

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>>  {
    std::env::set_var("RUST_LOG","actix_web=debug,actix_server=info,info");
    env_logger::init();
    println!("Hello, world!");
    let config_addr = ConfigActor::new().start();
    //naming start
    let listener_addr = InnerNamingListener::new_and_create(5000, None).await;
    let mut naming_addr = NamingActor::new(listener_addr).start();
    HttpServer::new(move || {
        let config_addr = config_addr.clone();
        let naming_addr = naming_addr.clone();
        App::new()
            .data(config_addr)
            .data(naming_addr)
            .wrap(middleware::Logger::default())
            .configure(app_config)
    })
    .workers(8)
    .bind("0.0.0.0:8848")?
    .run()
    .await?;
    Ok(())
}
