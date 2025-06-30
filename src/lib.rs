pub mod common;
pub mod config;
pub mod console;
pub mod grpc;
pub mod metrics;
pub mod namespace;
pub mod naming;
pub mod openapi;
pub mod raft;
pub mod starter;
pub mod user;
pub mod utils;
pub mod web_config;

pub mod health;
pub mod transfer;

pub mod ldap;

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

fn now_second_i32() -> i32 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i32
}

fn now_second_u32() -> u32 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as u32
}

#[cfg(test)]
mod tests {}
