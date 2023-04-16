
use actix_web::{App, web::Data};
use actix::{Actor};
use nacos_rust::grpc::bistream_manage::BiStreamManage;
use nacos_rust::grpc::handler::InvokerHandler;
use nacos_rust::grpc::nacos_proto::bi_request_stream_server::BiRequestStreamServer;
use nacos_rust::grpc::nacos_proto::request_server::RequestServer;
use nacos_rust::grpc::server::BiRequestStreamServerImpl;
use nacos_rust::naming::core::{NamingCmd, NamingResult};
use nacos_rust::{naming::core::NamingActor, grpc::server::RequestServerImpl};
use nacos_rust::config::config::{ConfigActor, ConfigCmd};
use tonic::transport::Server;
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
    //let naming_addr = NamingActor::new_and_create();
    let naming_addr = NamingActor::create_at_new_system();
    let naming_res: NamingResult = naming_addr.send(NamingCmd::QueryDalAddr).await.unwrap().unwrap();
    let naming_dal_addr = if let NamingResult::DalAddr(addr)=naming_res { 
        addr 
    } 
    else {
        panic!("error naming_dal_addr")
    };

    let mut bistream_manage = BiStreamManage::new();
    bistream_manage.set_config_addr(config_addr.clone());
    bistream_manage.set_naming_addr(naming_addr.clone());
    let bistream_manage_addr = bistream_manage.start();
    config_addr.do_send(ConfigCmd::SetConnManage(bistream_manage_addr.clone()));
    naming_addr.do_send(NamingCmd::SetConnManage(bistream_manage_addr.clone()));

    let mut invoker = InvokerHandler::new();
    invoker.add_config_handler(&config_addr);
    invoker.add_naming_handler(&naming_addr);



    tokio::spawn(async move {
        let addr = "0.0.0.0:9848".parse().unwrap();
        let request_server = RequestServerImpl::new(bistream_manage_addr.clone(),invoker);
        let bi_request_stream_server = BiRequestStreamServerImpl::new(bistream_manage_addr);
        Server::builder()
        .add_service(RequestServer::new(request_server))
        .add_service(BiRequestStreamServer::new(bi_request_stream_server))
        .serve(addr)
        .await.unwrap();
    });

    HttpServer::new(move || {
        let config_addr = config_addr.clone();
        let naming_addr = naming_addr.clone();
        let naming_dal_addr = naming_dal_addr.clone();
        App::new()
            .app_data(Data::new(config_addr))
            .app_data(Data::new(naming_addr))
            .app_data(Data::new(naming_dal_addr))
            .wrap(middleware::Logger::default())
            .configure(app_config)
    })
    .workers(8)
    .bind("0.0.0.0:8848")?
    .run()
    .await?;
    Ok(())
}
