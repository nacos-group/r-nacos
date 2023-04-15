
pub mod delay_notify;

#[derive(Default,Clone,Debug)]
pub struct NamingSysConfig {
    pub once_time_check_size:usize,
    pub service_time_out_millis:u64,
}

impl NamingSysConfig {
    pub fn new() -> Self {
        Self { 
            once_time_check_size: 10000, 
            service_time_out_millis: 30000 
        }
    }
}