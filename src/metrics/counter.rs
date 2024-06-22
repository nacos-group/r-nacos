use crate::metrics::metrics_key::MetricsKey;
use crate::metrics::model::CounterItem;
use std::collections::HashMap;

type Key = MetricsKey;

#[derive(Default, Debug)]
pub struct CounterManager {
    pub(crate) date_map: HashMap<Key, CounterItem>,
}

impl CounterManager {
    pub fn increment(&mut self, key: Key, value: u64) {
        if let Some(item) = self.date_map.get_mut(&key) {
            item.increment(value);
        } else {
            self.date_map.insert(key, value.into());
        }
    }

    pub fn absolute(&mut self, key: Key, value: u64) {
        if let Some(item) = self.date_map.get_mut(&key) {
            item.absolute(value);
        } else {
            self.date_map.insert(key, value.into());
        }
    }
    pub fn print_metrics(&self) {
        log::info!("-------------- COUNTER TYPE --------------");
        for (key, val) in &self.date_map {
            log::info!("[metrics_counter]|{}:{}|", key.get_key(), val.0);
        }
    }
}
