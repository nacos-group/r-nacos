pub(crate) mod auth;
pub mod common;
pub mod config;
pub mod console;
pub mod grpc;
pub mod naming;
pub mod raft;
pub mod starter;
pub mod user;
pub mod utils;
pub mod web_config;

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

#[cfg(test)]
mod tests {}
