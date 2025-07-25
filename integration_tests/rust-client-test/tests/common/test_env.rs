use std::env;

#[derive(Debug, Clone)]
pub struct TestEnvironment {
    pub standalone_host: String,
    pub cluster_hosts: Vec<String>,
    pub tenant: String,
    pub auth_info: Option<(String, String)>,
}

impl TestEnvironment {
    pub fn new() -> Self {
        let standalone_host =
            env::var("NACOS_HOST").unwrap_or_else(|_| "127.0.0.1:8848".to_string());

        let cluster_hosts = vec![
            env::var("NACOS_CLUSTER_HOST1").unwrap_or_else(|_| "127.0.0.1:8848".to_string()),
            env::var("NACOS_CLUSTER_HOST2").unwrap_or_else(|_| "127.0.0.1:8849".to_string()),
            env::var("NACOS_CLUSTER_HOST3").unwrap_or_else(|_| "127.0.0.1:8850".to_string()),
        ];

        let tenant = env::var("NACOS_TENANT").unwrap_or_else(|_| "public".to_string());

        let auth_info = if let Ok(username) = env::var("NACOS_USERNAME") {
            if let Ok(password) = env::var("NACOS_PASSWORD") {
                Some((username, password))
            } else {
                None
            }
        } else {
            None
        };

        Self {
            standalone_host,
            cluster_hosts,
            tenant,
            auth_info,
        }
    }

    pub fn get_standalone_addr(&self) -> String {
        self.standalone_host.clone()
    }

    pub fn get_cluster_addrs(&self) -> String {
        self.cluster_hosts.join(",")
    }

    pub fn get_cluster_host(&self, index: usize) -> Option<String> {
        self.cluster_hosts.get(index).cloned()
    }
}

impl Default for TestEnvironment {
    fn default() -> Self {
        Self::new()
    }
}
