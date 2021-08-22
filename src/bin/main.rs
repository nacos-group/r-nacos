
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
use nacos_rust::naming::udp_handler::{UdpSender,UdpReciver};
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

fn app_config(config:&mut web::ServiceConfig){
    config.service(
        web::resource("/").route(web::get().to(index))
    );
    cs_config(config);
    ns_config(config);
}

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>>  {
    std::env::set_var("RUST_LOG","actix_web=debug,actix_server=info");
    env_logger::init();
    println!("Hello, world!");
    let config_addr = ConfigActor::new().start();
    let config_addr2 = config_addr.clone();
    tokio::spawn(async move {
        loop {
            //println!("config timer");
            tokio::time::delay_for(tokio::time::Duration::from_millis(500)).await;
            config_addr2.send(ConfigCmd::PEEK_LISTENER_TIME_OUT).await;
        }
    });
    //naming start
    let local_addr:SocketAddr = "0.0.0.0:0".parse()?;
    let socket = UdpSocket::bind(local_addr).await?;
    //println!("udp local addr:{:?}",socket.local_addr());
    let (r,w) = socket.split();
    let sender_addr = UdpSender::new(w).start();
    let naming_addr = NamingActor::new(sender_addr).start();
    let naming_addr2 = naming_addr.clone();
    tokio::spawn(async move {
        loop {
            //println!("naming timer");
            tokio::time::delay_for(tokio::time::Duration::from_millis(5000)).await;
            naming_addr2.send(NamingCmd::PEEK_LISTENER_TIME_OUT).await;
        }
    });
    let naming_addr3 = naming_addr.clone();
    tokio::spawn(async move {
        UdpReciver::new(r,naming_addr3).recv().await;
    });
    HttpServer::new(move || {
        let config_addr = config_addr.clone();
        let naming_addr = naming_addr.clone();
        App::new()
            .data(config_addr)
            .data(naming_addr)
            .wrap(middleware::Logger::default())
            .configure(app_config)
    })
    .workers(4)
    .bind("0.0.0.0:8848")?
    .run()
    .await?;
    Ok(())
}
