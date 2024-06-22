use crate::metrics::metrics_type::MetricsType;
use crate::metrics::model::{CounterItem, HistogramItem};
use std::collections::HashMap;

type Key = MetricsType;

#[derive(Default, Debug)]
pub struct HistogramManager {
    pub(crate) date_map: HashMap<Key, HistogramItem>,
}

impl HistogramManager {
    pub fn init(&mut self, key: Key, bounds: &[f64]) {
        if !self.date_map.contains_key(&key) {
            if let Some(item) = HistogramItem::new(bounds) {
                self.date_map.insert(key, item);
            }
        }
    }

    pub fn record(&mut self, key: &Key, sample: f64) {
        if let Some(item) = self.date_map.get_mut(key) {
            item.record(sample);
        }
    }

    pub fn record_many(&mut self, key: &Key, samples: &[f64]) {
        if let Some(item) = self.date_map.get_mut(key) {
            item.record_many(samples);
        }
    }

    pub fn sum(&self, key: &Key) -> f64 {
        if let Some(item) = self.date_map.get(key) {
            item.sum
        } else {
            0f64
        }
    }

    pub fn count(&self, key: &Key) -> u64 {
        if let Some(item) = self.date_map.get(key) {
            item.count
        } else {
            0u64
        }
    }

    pub fn buckets(&self, key: &Key) -> Vec<(f64, u64)> {
        if let Some(item) = self.date_map.get(key) {
            item.buckets()
        } else {
            vec![]
        }
    }
    pub fn print_metrics(&self) {
        log::info!("-------------- HISTOGRAM TYPE --------------");
        for (key, val) in &self.date_map {
            log::info!("[metrics_histogram]|{}:{}|", key.get_key(), val);
        }
    }
}
