#![allow(unused_imports)]

use actix::Actor;
use actix_web::{web::Data, App};
use rnacos::common::AppSysConfig;
use rnacos::config::core::{ConfigActor, ConfigCmd};
use rnacos::grpc::bistream_manage::BiStreamManage;
use rnacos::grpc::handler::InvokerHandler;
use rnacos::grpc::nacos_proto::bi_request_stream_server::BiRequestStreamServer;
use rnacos::grpc::nacos_proto::request_server::RequestServer;
use rnacos::grpc::server::BiRequestStreamServerImpl;
use rnacos::naming::core::{NamingCmd, NamingResult};
use rnacos::{grpc::server::RequestServerImpl, naming::core::NamingActor};
use std::error::Error;
use std::sync::Arc;
use tonic::transport::Server;

use actix_web::{middleware, HttpServer};
use rnacos::web_config::app_config;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    std::env::set_var("RUST_LOG", "actix_web=debug,actix_server=info,info");
    dotenv::dotenv().ok();
    env_logger::init();
    let sys_config = AppSysConfig::init_from_env();
    let db = Arc::new(
        sled::Config::new()
            .path(&sys_config.config_db_dir)
            .mode(sled::Mode::LowSpace)
            .cache_capacity(10 * 1024 * 1024)
            //.flush_every_ms(Some(1000))
            .open()
            .unwrap(),
    );
    let http_addr = sys_config.get_http_addr();
    let grpc_addr = sys_config.get_grpc_addr();
    log::info!("http server addr:{}", &http_addr);
    log::info!("grpc server addr:{}", &grpc_addr);

    let config_addr = ConfigActor::new(db).start();
    //let naming_addr = NamingActor::new_and_create();
    let naming_addr = NamingActor::create_at_new_system();

    let mut bistream_manage = BiStreamManage::new();
    bistream_manage.set_config_addr(config_addr.clone());
    bistream_manage.set_naming_addr(naming_addr.clone());
    let bistream_manage_addr = bistream_manage.start();
    config_addr.do_send(ConfigCmd::SetConnManage(bistream_manage_addr.clone()));
    naming_addr.do_send(NamingCmd::SetConnManage(bistream_manage_addr.clone()));
    let bistream_manage_http_addr = bistream_manage_addr.clone();

    let mut invoker = InvokerHandler::new();
    invoker.add_config_handler(&config_addr);
    invoker.add_naming_handler(&naming_addr);

    tokio::spawn(async move {
        let addr = grpc_addr.parse().unwrap();
        let request_server = RequestServerImpl::new(bistream_manage_addr.clone(), invoker);
        let bi_request_stream_server = BiRequestStreamServerImpl::new(bistream_manage_addr);
        Server::builder()
            .add_service(RequestServer::new(request_server))
            .add_service(BiRequestStreamServer::new(bi_request_stream_server))
            .serve(addr)
            .await
            .unwrap();
    });

    let mut server = HttpServer::new(move || {
        let config_addr = config_addr.clone();
        let naming_addr = naming_addr.clone();
        let bistream_manage_http_addr = bistream_manage_http_addr.clone();
        //let naming_dal_addr = naming_dal_addr.clone();
        App::new()
            .app_data(Data::new(config_addr))
            .app_data(Data::new(naming_addr))
            .app_data(Data::new(bistream_manage_http_addr))
            //.app_data(Data::new(naming_dal_addr))
            .wrap(middleware::Logger::default())
            .configure(app_config)
    });
    if let Some(num) = sys_config.http_workers {
        server = server.workers(num);
    }
    server.bind(http_addr)?.run().await?;
    Ok(())
}
