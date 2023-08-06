#![allow(unused_imports)]

use std::collections::{BTreeMap, HashSet};
use std::time::Duration;
use actix::Actor;
use actix_web::{web::Data, App};
use async_raft::raft::ClientWriteRequest;
use async_raft::{Config, Raft, RaftStorage};
use rnacos::grpc::PayloadUtils;
use rnacos::raft::asyncraft::network::factory::{RaftConnectionFactory, RaftClusterRequestSender};
use rnacos::raft::cluster::model::RouterRequest;
use rnacos::raft::cluster::route::{RaftAddrRouter, ConfigRoute};
use rnacos::common::AppSysConfig;
use rnacos::config::core::{ConfigActor, ConfigCmd};
use rnacos::grpc::bistream_manage::BiStreamManage;
use rnacos::grpc::handler::InvokerHandler;
use rnacos::grpc::nacos_proto::bi_request_stream_server::BiRequestStreamServer;
use rnacos::grpc::nacos_proto::request_server::RequestServer;
use rnacos::grpc::server::BiRequestStreamServerImpl;
use rnacos::naming::core::{NamingCmd, NamingResult};
use rnacos::raft::asyncraft::network::network::RaftRouter;
use rnacos::raft::asyncraft::store::ClientRequest;
use rnacos::raft::asyncraft::store::store::AStore;
use rnacos::raft::network::httpnetwork::HttpNetworkFactory;
use rnacos::{grpc::server::RequestServerImpl, naming::core::NamingActor};
use sled::Db;
use std::error::Error;
use std::sync::Arc;
use tonic::transport::Server;

use actix_web::{middleware, HttpServer};
use rnacos::common::appdata::AppShareData;
use rnacos::raft::NacosRaft;
use rnacos::raft::network::GrpcNetworkFactory;
use rnacos::web_config::app_config;
use clap::Parser;

#[derive(Parser, Clone, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct AppOpt {
    /// env file path
    #[arg(short,long,default_value="")]
    pub env_file: String,
}

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

    let store = Arc::new(AStore::new(sys_config.raft_node_id.to_owned(),db,config_addr.clone()));
    let conn_factory = RaftConnectionFactory::new(60).start();
    let cluster_sender = Arc::new(RaftClusterRequestSender::new(conn_factory));
    let raft= build_raft(&sys_config,store.clone(),cluster_sender.clone()).await?;
    config_addr.do_send(ConfigCmd::SetRaft(raft.clone()));

    let raft_addr_router = Arc::new(RaftAddrRouter::new(raft.clone(),store.clone(),sys_config.raft_node_id.to_owned()));
    let config_route = Arc::new(ConfigRoute::new(config_addr.clone(),raft_addr_router,cluster_sender.clone()));

    let mut bistream_manage = BiStreamManage::new();
    bistream_manage.set_config_addr(config_addr.clone());
    bistream_manage.set_naming_addr(naming_addr.clone());
    let bistream_manage_addr = bistream_manage.start();
    config_addr.do_send(ConfigCmd::SetConnManage(bistream_manage_addr.clone()));
    naming_addr.do_send(NamingCmd::SetConnManage(bistream_manage_addr.clone()));
    let bistream_manage_http_addr = bistream_manage_addr.clone();

    let app_data = Arc::new(AppShareData{
        config_addr:config_addr.clone(),
        naming_addr:naming_addr.clone(),
        bi_stream_manage: bistream_manage_http_addr.clone(),
        raft:raft.clone(),
        raft_store:store,
        sys_config: sys_config.clone(),
        config_route,
        cluster_sender,
    });

    let mut invoker = InvokerHandler::new();
    invoker.add_config_handler(&app_data);
    invoker.add_naming_handler(&naming_addr);
    invoker.add_raft_handler(&app_data);

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

    let app_data = Data::new(app_data);

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
    let app_opt = AppOpt::parse();
    let env_path = app_opt.env_file.to_owned();
    //let env_path = std::env::var("RNACOS_ENV_FILE").unwrap_or_default();
    if env_path.is_empty() {
        dotenv::dotenv().ok();
    } else {
        dotenv::from_path(env_path).ok();
    }
}

async fn build_raft(sys_config: &Arc<AppSysConfig>,store:Arc<AStore>,cluster_sender:Arc<RaftClusterRequestSender>) -> anyhow::Result<Arc<NacosRaft>> {
    let config = Config::build("rnacos raft".to_owned())
        .heartbeat_interval(500) 
        .election_timeout_min(1500) 
        .election_timeout_max(3000) 
        .validate().unwrap();
    let config = Arc::new(config);
    let network = Arc::new(RaftRouter::new(store.clone(),cluster_sender.clone()));
    let raft = Arc::new(Raft::new(sys_config.raft_node_id.to_owned(),config.clone(),network,store.clone()));
    if sys_config.raft_auto_init {
        tokio::spawn(auto_init_raft(store, raft.clone(),sys_config.clone()));
    }
    else if !sys_config.raft_join_addr.is_empty() {
        tokio::spawn(auto_join_raft(store,sys_config.clone(),cluster_sender));
    }
    Ok(raft)
}

async fn auto_init_raft(store:Arc<AStore>,raft:Arc<NacosRaft>,sys_config: Arc<AppSysConfig>) -> anyhow::Result<()> {
    let state = store.get_initial_state().await?;
    if state.last_log_term==0 && state.last_log_index==0 {
        log::info!("auto init raft. node_id:{},addr:{}",&sys_config.raft_node_id,&sys_config.raft_node_addr);
        let mut members = HashSet::new();
        members.insert(sys_config.raft_node_id.to_owned());
        raft.initialize(members).await.ok();
        raft.client_write(ClientWriteRequest::new(ClientRequest::NodeAddr { id:sys_config.raft_node_id, addr: Arc::new(sys_config.raft_node_addr.to_owned())})).await.ok();
    }
    Ok(())
}

async fn auto_join_raft(store:Arc<AStore>,sys_config: Arc<AppSysConfig>,cluster_sender:Arc<RaftClusterRequestSender>) -> anyhow::Result<()> {
    let state = store.get_initial_state().await?;
    if state.last_log_term==0 && state.last_log_index==0 {
        //wait for self raft network started
        tokio::time::sleep(Duration::from_millis(500)).await;
        let req = RouterRequest::JoinNode { node_id: sys_config.raft_node_id.to_owned(), node_addr: Arc::new(sys_config.raft_node_addr.to_owned())};
        let request = serde_json::to_string(&req).unwrap_or_default();
        let payload = PayloadUtils::build_payload("RaftRouteRequest", request);
        cluster_sender.send_request(Arc::new(sys_config.raft_join_addr.to_owned()), payload).await?;
        log::info!("auto join raft,join_addr:{}.node_id:{},addr:{}",&sys_config.raft_join_addr,&sys_config.raft_node_id,&sys_config.raft_node_addr);
    }
    Ok(())
}
