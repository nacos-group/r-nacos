use chrono::{DateTime, FixedOffset, Utc};
use std::time::SystemTime;

const DATETIME_TIMESTAMP_FMT: &str = "%Y-%m-%dT%H:%M:%S%.3f%:z";

pub fn get_now_timestamp_str(offset: &FixedOffset) -> String {
    DateTime::<Utc>::from(SystemTime::now())
        .with_timezone(offset)
        .format(DATETIME_TIMESTAMP_FMT)
        .to_string()
}
