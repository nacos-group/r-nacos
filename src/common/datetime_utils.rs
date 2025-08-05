use chrono::{DateTime, FixedOffset, Utc};
use std::time::SystemTime;

const DATETIME_TIMESTAMP_FMT: &str = "%Y-%m-%dT%H:%M:%S%.3f%:z";

pub fn now_millis() -> u64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

pub fn now_millis_i64() -> i64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

pub fn now_second_i32() -> i32 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i32
}

pub fn now_second_u32() -> u32 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as u32
}

pub fn get_now_timestamp_str(offset: &FixedOffset) -> String {
    DateTime::<Utc>::from(SystemTime::now())
        .with_timezone(offset)
        .format(DATETIME_TIMESTAMP_FMT)
        .to_string()
}
