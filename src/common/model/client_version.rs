use std::fmt::Display;

#[derive(Default, Debug, Clone)]
pub struct ClientVersion {
    pub client: ClientNameType,
    pub version: String,
}

impl ClientVersion {
    pub fn new(client: &str, version: String) -> Self {
        Self {
            client: ClientNameType::from_string(client),
            version,
        }
    }

    pub fn from_string(s: &str) -> Self {
        let parts = s.split(':').collect::<Vec<_>>();
        if parts.len() < 2 {
            Self::default()
        } else {
            Self::new(parts[0], parts[1].to_string())
        }
    }
}

impl Display for ClientVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.client.name(), self.version)
    }
}

#[derive(Debug, Clone)]
pub enum ClientNameType {
    NacosJavaClient,
    NacosGoClient,
    NacosPythonClient,
    NacosRustClient,
    Unknown(String),
}

impl ClientNameType {
    pub fn from_string(s: &str) -> Self {
        match s {
            "Nacos-Java-Client" => ClientNameType::NacosJavaClient,
            "Nacos-Python-Client" => ClientNameType::NacosPythonClient,
            "Nacos-Go-Client" => ClientNameType::NacosGoClient,
            "Nacos-Rust-Client" => ClientNameType::NacosRustClient,
            _ => ClientNameType::Unknown(s.to_string()),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            ClientNameType::NacosJavaClient => "Nacos-Java-Client",
            ClientNameType::NacosGoClient => "Nacos-Go-Client",
            ClientNameType::NacosPythonClient => "Nacos-Python-Client",
            ClientNameType::NacosRustClient => "Nacos-Rust-Client",
            ClientNameType::Unknown(s) => s,
        }
    }

    pub fn is_unknown(&self) -> bool {
        matches!(self, ClientNameType::Unknown(_))
    }

    pub fn is_java_sdk(&self) -> bool {
        matches!(self, ClientNameType::NacosJavaClient)
    }

    pub fn is_go_sdk(&self) -> bool {
        matches!(self, ClientNameType::NacosGoClient)
    }

    pub fn is_python_sdk(&self) -> bool {
        matches!(self, ClientNameType::NacosPythonClient)
    }

    pub fn is_rust_sdk(&self) -> bool {
        matches!(self, ClientNameType::NacosRustClient)
    }
}

impl Default for ClientNameType {
    fn default() -> Self {
        ClientNameType::Unknown("".to_string())
    }
}
