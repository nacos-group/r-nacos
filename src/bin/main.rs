
use actix_web::App;
use actix::Actor;
use nacos_rust::naming::core::NamingActor;
use nacos_rust::config::config::ConfigActor;
use std::error::Error;

use nacos_rust::web_config::app_config;
use actix_web::{
    middleware,HttpServer,
};

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>>  {
    std::env::set_var("RUST_LOG","actix_web=debug,actix_server=info,info");
    env_logger::init();
    let config_addr = ConfigActor::new().start();
    let naming_addr = NamingActor::new_and_create(5000);
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
