use std::collections::HashMap;
use crate::metrics::GaugeItem;
use crate::metrics::metrics_type::MetricsType;

type key = MetricsType;

#[derive(Default,Debug)]
pub struct GaugeManager{
    date_map : HashMap<key,GaugeItem>,
}

impl GaugeManager {
    pub fn increment(&mut self, key: key, value:f64) {
        if let Some(item) = self.date_map.get_mut(&key) {
            item.increment(value);
        }
        else {
            self.date_map.insert(key,value.into());
        }
    }

    pub fn decrement(&mut self, key: key, value:f64) {
        if let Some(item) = self.date_map.get_mut(&key) {
            item.increment(value);
        }
        else {
            self.date_map.insert(key,0f64.into());
        }
    }

    pub fn set(&mut self, key: key, value:f64) {
        if let Some(item) = self.date_map.get_mut(&key) {
            item.set(value);
        }
        else {
            self.date_map.insert(key,value.into());
        }
    }
}
