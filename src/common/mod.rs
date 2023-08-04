use lazy_static::lazy_static;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub mod appdata;
pub mod byte_utils;
pub mod cycle_queue;
pub mod delay_notify;
pub mod rusqlite_utils;
pub mod sled_utils;
pub mod string_utils;

lazy_static! {
    // Global app sys config
    pub static ref APP_SYS_CONFIG: AppSysConfig = AppSysConfig::init_from_env();
    // Global sled db
    pub static ref DB: Arc<Mutex<sled::Db>> = Arc::new(Mutex::new(
        sled::Config::new()
            .path(&APP_SYS_CONFIG.config_db_dir)
            .mode(sled::Mode::HighThroughput)
            .open()
            .unwrap()
    ));
}

#[derive(Default, Clone, Debug)]
pub struct NamingSysConfig {
    pub once_time_check_size: usize,
    pub service_time_out_millis: u64,
    pub instance_metadata_time_out_millis: u64,
}

impl NamingSysConfig {
    pub fn new() -> Self {
        Self {
            once_time_check_size: 10000,
            service_time_out_millis: 30000,
            instance_metadata_time_out_millis: 60000,
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct AppSysConfig {
    pub config_db_file: String,
    pub config_db_dir: String,
    pub http_port: u16,
    pub http_workers: Option<usize>,
    pub grpc_port: u16,
    pub raft_node_id: u64,
    pub raft_node_addr: String,
    pub raft_auto_init: bool,
    pub raft_join_addr: String,
}

impl AppSysConfig {
    pub fn init_from_env() -> Self {
        let config_db_file =
            std::env::var("RNACOS_CONFIG_DB_FILE").unwrap_or("config.db".to_owned());
        let http_port = std::env::var("RNACOS_HTTP_PORT")
            .unwrap_or("8848".to_owned())
            .parse()
            .unwrap_or(8848);
        let http_workers = std::env::var("RNACOS_HTTP_WORKERS")
            .unwrap_or("".to_owned())
            .parse()
            .ok();
        let grpc_port = std::env::var("RNACOS_GRPC_PORT")
            .unwrap_or("".to_owned())
            .parse()
            .unwrap_or(http_port + 1000);
        let config_db_dir = std::env::var("RNACOS_CONFIG_DB_DIR").unwrap_or("nacos_db".to_owned());
        let raft_node_id = std::env::var("RNACOS_RAFT_NODE_ID")
            .unwrap_or("1".to_owned())
            .parse()
            .unwrap_or(1);
        let raft_node_addr =
            std::env::var("RNACOS_RAFT_NODE_ADDR").unwrap_or(format!("127.0.0.1:{}", &http_port));
        let raft_auto_init = std::env::var("RNACOS_RAFT_AUTO_INIT")
            .unwrap_or("".to_owned())
            .parse()
            .unwrap_or(raft_node_id == 1);
        let raft_join_addr =
            std::env::var("RNACOS_RAFT_JOIN_ADDR").unwrap_or_default();
        Self {
            config_db_dir,
            config_db_file,
            http_port,
            grpc_port,
            http_workers,
            raft_node_id,
            raft_node_addr,
            raft_auto_init,
            raft_join_addr,
        }
    }

    pub fn get_grpc_addr(&self) -> String {
        format!("0.0.0.0:{}", &self.grpc_port)
    }

    pub fn get_http_addr(&self) -> String {
        format!("0.0.0.0:{}", &self.http_port)
    }
}

/**
 * generate uuid in i64
 */
pub fn gen_uuid() -> i64 {
    let uuid = Uuid::new_v4();
    let bytes = uuid.as_bytes();
    let msb = u64::from_be_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]);
    let lsb = u64::from_be_bytes([
        bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
    ]);

    ((msb << 32) | lsb) as i64
}
