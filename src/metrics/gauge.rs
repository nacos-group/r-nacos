use crate::metrics::metrics_key::MetricsKey;
use crate::metrics::model::GaugeItem;
use std::collections::HashMap;

type Key = MetricsKey;

#[derive(Default, Debug)]
pub struct GaugeManager {
    pub(crate) date_map: HashMap<Key, GaugeItem>,
}

impl GaugeManager {
    pub fn increment(&mut self, key: Key, value: f64) {
        if let Some(item) = self.date_map.get_mut(&key) {
            item.increment(value);
        } else {
            self.date_map.insert(key, value.into());
        }
    }

    pub fn decrement(&mut self, key: Key, value: f64) {
        if let Some(item) = self.date_map.get_mut(&key) {
            item.increment(value);
        } else {
            self.date_map.insert(key, 0f64.into());
        }
    }

    pub fn set(&mut self, key: Key, value: f64) {
        if let Some(item) = self.date_map.get_mut(&key) {
            item.set(value);
        } else {
            self.date_map.insert(key, value.into());
        }
    }

    pub fn print_metrics(&self) {
        log::info!("-------------- GAUGE TYPE --------------");
        for (key, val) in &self.date_map {
            log::info!("[metrics_gauge]|{}:{}|", key.get_key(), val.0);
        }
    }
}
