use crate::metrics::metrics_type::MetricsType;
use crate::metrics::model::GaugeItem;
use std::collections::HashMap;

type Key = MetricsType;

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
}
