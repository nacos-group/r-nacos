#![allow(unused_imports)]

use actix::Actor;
use actix_web::{web::Data, App};
use async_raft_ext::raft::ClientWriteRequest;
use async_raft_ext::{Config, Raft, RaftStorage};
use rnacos::common::AppSysConfig;
use rnacos::config::core::{ConfigActor, ConfigCmd};
use rnacos::console::middle::login_middle::CheckLogin;
use rnacos::grpc::bistream_manage::BiStreamManage;
use rnacos::grpc::handler::InvokerHandler;
use rnacos::grpc::nacos_proto::bi_request_stream_server::BiRequestStreamServer;
use rnacos::grpc::nacos_proto::request_server::RequestServer;
use rnacos::grpc::server::BiRequestStreamServerImpl;
use rnacos::grpc::PayloadUtils;
use rnacos::naming::core::{NamingCmd, NamingResult};
use rnacos::raft::cluster::model::RouterRequest;
use rnacos::raft::cluster::route::{ConfigRoute, RaftAddrRouter};
use rnacos::raft::network::core::RaftRouter;
use rnacos::raft::network::factory::{RaftClusterRequestSender, RaftConnectionFactory};
use rnacos::raft::store::ClientRequest;
use rnacos::starter::{build_share_data, config_factory};
use rnacos::{grpc::server::RequestServerImpl, naming::core::NamingActor, openapi};
use sled::Db;
use std::collections::{BTreeMap, HashSet};
use std::error::Error;
use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;
use tonic::transport::Server;

use actix_web::{middleware, HttpServer};
use clap::Parser;
use env_logger::TimestampPrecision;
use env_logger_timezone_fmt::{TimeZoneFormat, TimeZoneFormatEnv};
use rnacos::common::appdata::AppShareData;
use rnacos::common::constant::APP_VERSION;
use rnacos::raft::NacosRaft;
use rnacos::web_config::{app_config, console_config};

#[derive(Parser, Clone, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct AppOpt {
    /// env file path
    #[arg(short, long, default_value = "")]
    pub env_file: String,
}

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    init_env();
    let rust_log = std::env::var("RUST_LOG").unwrap_or("info".to_owned());
    println!("version:{}, RUST_LOG:{}", APP_VERSION, &rust_log);
    std::env::set_var("RUST_LOG", &rust_log);
    let sys_config = Arc::new(AppSysConfig::init_from_env());
    let timezone_fmt = Arc::new(TimeZoneFormatEnv::new(
        sys_config.gmt_fixed_offset_hours.map(|v| v * 60 * 60),
        Some(TimestampPrecision::Micros),
    ));
    env_logger::Builder::from_default_env()
        .format(move |buf, record| TimeZoneFormat::new(buf, &timezone_fmt).write(record))
        .init();
    let factory_data = config_factory(sys_config.clone()).await?;
    let app_data = build_share_data(factory_data.clone())?;
    let http_addr = sys_config.get_http_addr();
    let grpc_addr = sys_config.get_grpc_addr();
    log::info!("http server addr:{}", &http_addr);
    log::info!("grpc server addr:{}", &grpc_addr);

    let mut invoker = InvokerHandler::new();
    invoker.add_config_handler(&app_data);
    invoker.add_naming_handler(&app_data);
    invoker.add_raft_handler(&app_data);

    let grpc_app_data = app_data.clone();

    tokio::spawn(async move {
        let addr = grpc_addr.parse().unwrap();
        let request_server =
            RequestServerImpl::new(grpc_app_data.bi_stream_manage.clone(), invoker);
        let bi_request_stream_server =
            BiRequestStreamServerImpl::new(grpc_app_data.bi_stream_manage.clone());
        Server::builder()
            .add_service(RequestServer::new(request_server))
            .add_service(BiRequestStreamServer::new(bi_request_stream_server))
            .serve(addr)
            .await
            .unwrap();
    });

    let app_console_data = app_data.clone();
    let app_data = Data::new(app_data);

    if sys_config.http_console_port > 0 {
        std::thread::spawn(move || {
            actix_rt::System::with_tokio_rt(|| {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
            })
            .block_on(run_console_web(app_console_data));
        });
    }

    let mut server = HttpServer::new(move || {
        let config_addr = app_data.config_addr.clone();
        let naming_addr = app_data.naming_addr.clone();
        let bistream_manage_http_addr = app_data.bi_stream_manage.clone();
        let app_data = app_data.clone();
        let app_config_shard = app_data.sys_config.deref().clone();
        App::new()
            .app_data(app_data)
            .app_data(Data::new(config_addr))
            .app_data(Data::new(naming_addr))
            .app_data(Data::new(bistream_manage_http_addr))
            .wrap(middleware::Logger::default())
            .configure(app_config(app_config_shard))
    });
    if let Some(num) = sys_config.http_workers {
        server = server.workers(num);
    }
    println!("rnacos started");
    server.bind(http_addr)?.run().await?;
    Ok(())
}

fn init_env() {
    let app_opt = AppOpt::parse();
    let env_path = app_opt.env_file;
    //let env_path = std::env::var("RNACOS_ENV_FILE").unwrap_or_default();
    if env_path.is_empty() {
        dotenv::dotenv().ok();
    } else {
        dotenv::from_path(env_path).ok();
    }
}

async fn run_console_web(source_app_data: Arc<AppShareData>) {
    let http_console_addr = source_app_data.sys_config.get_http_console_addr();
    log::info!("new console server http addr:{}", &http_console_addr);
    let app_data = Data::new(source_app_data.clone());
    HttpServer::new(move || {
        let source_app_data = source_app_data.clone();
        let config_addr = app_data.config_addr.clone();
        let naming_addr = app_data.naming_addr.clone();
        let bistream_manage_http_addr = app_data.bi_stream_manage.clone();
        let app_data = app_data.clone();
        App::new()
            .app_data(app_data)
            .app_data(Data::new(config_addr))
            .app_data(Data::new(naming_addr))
            .app_data(Data::new(bistream_manage_http_addr))
            .wrap(CheckLogin::new(source_app_data))
            .wrap(middleware::Logger::default())
            .wrap(middleware::Compress::default())
            .configure(console_config)
    })
    .workers(2)
    .bind(http_console_addr)
    .unwrap()
    .run()
    .await
    .ok();
}
