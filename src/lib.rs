
pub mod config;
pub mod naming;
pub(crate) mod auth;
pub mod web_config;
pub mod utils;
pub mod rusqlite_utils;
pub mod grpc;
pub(crate) mod common;

pub use inner_mem_cache::TimeoutSet;


fn now_millis() -> u64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

fn now_millis_i64() -> i64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}