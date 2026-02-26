use chrono::{DateTime, FixedOffset, Utc};
use std::time::SystemTime;

const DATETIME_TIMESTAMP_FMT: &str = "%Y-%m-%dT%H:%M:%S%.3f%:z";

/// Returns the duration since UNIX_EPOCH. Panics only if system clock is before epoch.
#[inline]
fn since_epoch() -> std::time::Duration {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("system clock is before UNIX epoch")
}

pub fn now_millis() -> u64 {
    since_epoch().as_millis() as u64
}

pub fn now_millis_i64() -> i64 {
    since_epoch().as_millis() as i64
}

pub fn now_second_i32() -> i32 {
    since_epoch().as_secs() as i32
}

pub fn now_second_u32() -> u32 {
    since_epoch().as_secs() as u32
}

pub fn get_now_timestamp_str(offset: &FixedOffset) -> String {
    DateTime::<Utc>::from(SystemTime::now())
        .with_timezone(offset)
        .format(DATETIME_TIMESTAMP_FMT)
        .to_string()
}
