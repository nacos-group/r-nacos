#![allow(unused_imports, unreachable_code)]
use nacos_rust_client::client::naming_client::{InstanceDefaultListener, ServiceInstanceKey};
use nacos_rust_client::conn_manage;
use std::sync::Arc;

use std::time::Duration;

use nacos_rust_client::client::naming_client::{Instance, NamingClient, QueryInstanceListParams};
use nacos_rust_client::client::{AuthInfo, ClientBuilder, HostInfo};

pub(crate) const CONN_COUNT: usize = 100;

#[tokio::main]
async fn main() {
    std::env::set_var("RUST_LOG", "INFO");
    env_logger::init();
    let namespace_id = "public".to_owned();
    let mut list = vec![];
    for i in 0..CONN_COUNT {
        let auth_info = None;
        let client = ClientBuilder::new()
            .set_endpoint_addrs("127.0.0.1:8848")
            .set_auth_info(auth_info)
            .set_tenant(namespace_id.clone())
            .set_use_grpc(true)
            .build_naming_client();
        list.push(client);
    }
    tokio::signal::ctrl_c()
        .await
        .expect("failed to listen for event");
}
