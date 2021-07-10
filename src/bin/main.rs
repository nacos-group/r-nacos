
use nacos_rust::config::config::{
    ConfigActor,ConfigCmd,ConfigKey,ConfigResult,ListenerItem,ListenerResult
};
use nacos_rust::config::api::{
    app_config as cs_config
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

fn app_config(config:&mut web::ServiceConfig){
    config.service(
        web::resource("/").route(web::get().to(index))
    );
    cs_config(config);
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let config_addr = ConfigActor::new().start();
    let config_addr2 = config_addr.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::delay_for(tokio::time::Duration::from_millis(500)).await;
            config_addr2.send(ConfigCmd::PEEK_LISTENER_TIME_OUT).await;
        }
    });
    std::env::set_var("RUST_LOG","actix_web=debug,actix_server=info");
    env_logger::init();
    println!("Hello, world!");
    HttpServer::new(move || {
        let config_addr = config_addr.clone();
        App::new()
            .data(config_addr)
            .wrap(middleware::Logger::default())
            .configure(app_config)
    })
    .workers(4)
    .bind("0.0.0.0:8848")?
    .run()
    .await
}
