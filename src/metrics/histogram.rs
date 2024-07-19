use crate::metrics::metrics_key::{MetricsKey, ORDER_ALL_KEYS};
use crate::metrics::model::{HistogramValue, HistogramValueFmtWrap};
use bytes::BytesMut;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::Write;

type Key = MetricsKey;

#[derive(Default, Debug)]
pub struct HistogramManager {
    pub(crate) data_map: HashMap<Key, HistogramValue>,
}

impl HistogramManager {
    pub fn init(&mut self, key: Key, bounds: &[f32]) {
        if let Entry::Vacant(e) = self.data_map.entry(key) {
            if let Some(item) = HistogramValue::new(bounds) {
                e.insert(item);
            }
        }
    }

    pub fn get_value(&self, key: &Key) -> Option<&HistogramValue> {
        self.data_map.get(key)
    }

    pub fn record(&mut self, key: &Key, sample: f32) {
        if let Some(item) = self.data_map.get_mut(key) {
            item.record(sample);
        }
    }

    pub fn record_many(&mut self, key: &Key, samples: &[f32]) {
        if let Some(item) = self.data_map.get_mut(key) {
            item.record_many(samples);
        }
    }

    pub fn sum(&self, key: &Key) -> f32 {
        if let Some(item) = self.data_map.get(key) {
            item.sum
        } else {
            0f32
        }
    }

    pub fn count(&self, key: &Key) -> u64 {
        if let Some(item) = self.data_map.get(key) {
            item.count
        } else {
            0u64
        }
    }

    pub fn buckets(&self, key: &Key) -> Vec<(f32, u64)> {
        if let Some(item) = self.data_map.get(key) {
            item.buckets()
        } else {
            vec![]
        }
    }
    pub fn print_metrics(&self) {
        //log::info!("-------------- METRICS HISTOGRAM --------------");
        for key in ORDER_ALL_KEYS.iter() {
            if let Some(val) = self.data_map.get(key) {
                log::info!("[metrics_histogram]|{}:{}|", key.get_key(), val);
            }
        }
    }

    pub fn export(&mut self, bytes_mut: &mut BytesMut) -> anyhow::Result<()> {
        for (key, value) in self.data_map.iter() {
            bytes_mut.write_str(&format!("{}", &HistogramValueFmtWrap::new(key, value)))?;
        }
        Ok(())
    }
}
