#![allow(unused_imports)]

use std::collections::BTreeMap;
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
use sled::Db;
use std::error::Error;
use std::sync::Arc;
use tonic::transport::Server;

use actix_web::{middleware, HttpServer};
use openraft::{Config, Raft};
use rnacos::common::appdata::AppData;
use rnacos::raft::store::store::Store;
use rnacos::raft::NacosRaft;
use rnacos::raft::network::NetworkFactory;
use rnacos::web_config::app_config;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    std::env::set_var("RUST_LOG", "actix_web=debug,actix_server=info,info");
    init_env();
    env_logger::builder().format_timestamp_micros().init();
    let sys_config = Arc::new(AppSysConfig::init_from_env());
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

    let config_addr = ConfigActor::new(db.clone()).start();
    //let naming_addr = NamingActor::new_and_create();
    let naming_addr = NamingActor::create_at_new_system();

    let store = Arc::new(Store::new(db,config_addr.clone()));
    let raft= build_raft(&sys_config,store.clone()).await?;

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
    invoker.add_raft_handler(&raft);

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

    let app_data = Data::new(AppData{
        config_addr:config_addr.clone(),
        naming_addr:naming_addr.clone(),
        bi_stream_manage: bistream_manage_http_addr.clone(),
        raft,
        raft_store:store,
        sys_config: sys_config.clone(),
    });

    let mut server = HttpServer::new(move || {
        let config_addr = config_addr.clone();
        let naming_addr = naming_addr.clone();
        let bistream_manage_http_addr = bistream_manage_http_addr.clone();
        let app_data = app_data.clone();
        //let naming_dal_addr = naming_dal_addr.clone();
        App::new()
            .app_data(app_data)
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

fn init_env() {
    let env_path = std::env::var("RNACOS_ENV_FILE").unwrap_or_default();
    if env_path.is_empty() {
        dotenv::dotenv().ok();
    } else {
        dotenv::from_path(env_path).ok();
    }
}

async fn build_raft(sys_config: &Arc<AppSysConfig>,store:Arc<Store>) -> anyhow::Result<NacosRaft> {
    let config = Config {
        heartbeat_interval: 500,
        election_timeout_min: 1500,
        election_timeout_max: 3000,
        ..Default::default()
    };
    let config = Arc::new(config.validate()?);
    let network = NetworkFactory {};
    let raft = Raft::new(sys_config.raft_node_id.to_owned(), config.clone(), network, store.clone()).await.unwrap();
    if sys_config.raft_auto_init {
        let mut nodes = BTreeMap::new();
        nodes.insert(sys_config.raft_node_id.to_owned(), openraft::BasicNode { addr: sys_config.raft_node_addr.clone() });
        raft.initialize(nodes).await.ok();
    }
    Ok(raft)
}
