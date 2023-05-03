
pub mod delay_notify;
pub mod rusqlite_utils;
pub mod string_utils;

#[derive(Default,Clone,Debug)]
pub struct NamingSysConfig {
    pub once_time_check_size:usize,
    pub service_time_out_millis:u64,
    pub instance_metadata_time_out_millis:u64,
}

impl NamingSysConfig {
    pub fn new() -> Self {
        Self { 
            once_time_check_size: 10000, 
            service_time_out_millis: 30000 ,
            instance_metadata_time_out_millis: 60000,
        }
    }
}

#[derive(Default,Clone,Debug)]
pub struct AppSysConfig{
    pub config_db_file:String,
    pub http_port:u32,
    pub grpc_port:u32,
}

impl AppSysConfig {
    pub fn init_from_env() -> Self {
        let config_db_file=std::env::var("RNACOS_CONFIG_DB_FILE").unwrap_or("config.db".to_owned());
        let http_port=std::env::var("RNACOS_HTTP_PORT")
            .unwrap_or("8848".to_owned())
            .parse().unwrap_or(8848);
        let grpc_port = std::env::var("RNACOS_GRPC_PORT")
            .unwrap_or("".to_owned())
            .parse().unwrap_or(http_port+1000);
        Self { 
            config_db_file, 
            http_port,
            grpc_port 
        }
    }

    pub fn get_grpc_addr(&self) -> String {
        format!("0.0.0.0:{}",&self.grpc_port)
    }

    pub fn get_http_addr(&self) -> String {
        format!("0.0.0.0:{}",&self.http_port)
    }
}