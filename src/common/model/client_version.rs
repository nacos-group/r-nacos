#[derive(Default, Debug, Clone)]
pub struct ClientVersion {
    pub client: ClientNameType,
    pub version: String,
}

impl ClientVersion {
    pub fn new(client: &str, version: String) -> Self {
        Self {
            client: ClientNameType::from_str(client),
            version,
        }
    }

    pub fn from_str(s: &str) -> Self {
        let parts = s.split(':').collect::<Vec<_>>();
        if parts.len() < 2 {
            Self::default()
        } else {
            Self::new(parts[0], parts[1].to_string())
        }
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
    pub fn from_str(s: &str) -> Self {
        match s {
            "Nacos-Java-Client" => ClientNameType::NacosJavaClient,
            "Nacos-Python-Client" => ClientNameType::NacosGoClient,
            "Nacos-Go-Client" => ClientNameType::NacosPythonClient,
            "Nacos-Rust-Client" => ClientNameType::NacosRustClient,
            _ => ClientNameType::Unknown(s.to_string()),
        }
    }

    pub fn is_unknown(&self) -> bool {
        match self {
            ClientNameType::Unknown(_) => true,
            _ => false,
        }
    }

    pub fn is_java_sdk(&self) -> bool {
        match self {
            ClientNameType::NacosJavaClient => true,
            _ => false,
        }
    }

    pub fn is_go_sdk(&self) -> bool {
        match self {
            ClientNameType::NacosGoClient => true,
            _ => false,
        }
    }

    pub fn is_python_sdk(&self) -> bool {
        match self {
            ClientNameType::NacosPythonClient => true,
            _ => false,
        }
    }

    pub fn is_rust_sdk(&self) -> bool {
        match self {
            ClientNameType::NacosRustClient => true,
            _ => false,
        }
    }
}

impl Default for ClientNameType {
    fn default() -> Self {
        ClientNameType::Unknown("".to_string())
    }
}
