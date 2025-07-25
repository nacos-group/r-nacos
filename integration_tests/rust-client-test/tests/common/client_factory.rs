use std::sync::Arc;

use super::test_env::TestEnvironment;
use nacos_rust_client::client::config_client::ConfigClient;
use nacos_rust_client::client::naming_client::NamingClient;
use nacos_rust_client::client::{AuthInfo, ClientBuilder};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Http,
    Grpc,
}

#[derive(Debug, Clone)]
pub struct ClientFactory {
    env: TestEnvironment,
}

impl ClientFactory {
    pub fn new() -> Self {
        Self {
            env: TestEnvironment::new(),
        }
    }

    pub fn create_config_client(
        &self,
        protocol: Protocol,
        use_cluster: bool,
        cluster_node: Option<usize>,
    ) -> Arc<ConfigClient> {
        let addrs = if use_cluster {
            match cluster_node {
                Some(node) => self
                    .env
                    .get_cluster_host(node)
                    .unwrap_or_else(|| self.env.get_standalone_addr()),
                None => self.env.get_cluster_addrs(),
            }
        } else {
            self.env.get_standalone_addr()
        };

        let mut builder = ClientBuilder::new()
            .set_endpoint_addrs(&addrs)
            .set_tenant(self.env.tenant.to_string());

        if let Some((username, password)) = &self.env.auth_info {
            builder = builder.set_auth_info(Some(AuthInfo::new(username, password)));
        }

        match protocol {
            Protocol::Http => builder.set_use_grpc(false),
            Protocol::Grpc => builder.set_use_grpc(true),
        }
        .build_config_client()
    }

    pub fn create_naming_client(
        &self,
        protocol: Protocol,
        use_cluster: bool,
        cluster_node: Option<usize>,
    ) -> Arc<NamingClient> {
        let addrs = if use_cluster {
            match cluster_node {
                Some(node) => self
                    .env
                    .get_cluster_host(node)
                    .unwrap_or_else(|| self.env.get_standalone_addr()),
                None => self.env.get_cluster_addrs(),
            }
        } else {
            self.env.get_standalone_addr()
        };

        let mut builder = ClientBuilder::new()
            .set_endpoint_addrs(&addrs)
            .set_tenant(self.env.tenant.to_string());

        if let Some((username, password)) = &self.env.auth_info {
            builder = builder.set_auth_info(Some(AuthInfo::new(username, password)));
        }

        match protocol {
            Protocol::Http => builder.set_use_grpc(false),
            Protocol::Grpc => builder.set_use_grpc(true),
        }
        .build_naming_client()
    }
}

impl Default for ClientFactory {
    fn default() -> Self {
        Self::new()
    }
}
