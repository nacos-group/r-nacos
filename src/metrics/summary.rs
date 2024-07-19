use crate::metrics::metrics_key::MetricsKey;
use crate::metrics::model::{HistogramValue, SummaryValue, SummaryValueFmtWrap};
use bytes::BytesMut;
use lazy_static::lazy_static;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::Write;

type Key = MetricsKey;

lazy_static! {
    /// 用于有序遍历打印信息
    pub static ref DEFAULT_SUMMARY_BOUNDS: Vec<f32> = vec![0.5f32, 0.6f32, 0.7f32, 0.8f32, 0.9f32, 0.95f32, 1f32];
}

#[derive(Default, Debug)]
pub struct SummaryManager {
    pub(crate) data_map: HashMap<Key, SummaryValue>,
}

impl SummaryManager {
    pub fn init(&mut self, key: Key, bounds: &[f32]) {
        if let Entry::Vacant(e) = self.data_map.entry(key) {
            e.insert(SummaryValue::new(bounds));
        }
    }

    pub fn recalculate_from_histogram(&mut self, key: &Key, histogram_value: &HistogramValue) {
        if let Some(item) = self.data_map.get_mut(key) {
            item.recalculate_from_histogram(histogram_value)
        }
    }

    pub fn print_metrics(&self) {
        for (key, value) in self.data_map.iter() {
            log::info!("[metrics_summary]|{}:{}|", key.get_key(), value);
        }
    }

    pub fn export(&mut self, bytes_mut: &mut BytesMut) -> anyhow::Result<()> {
        for (key, value) in self.data_map.iter() {
            bytes_mut.write_str(&format!("{}", &SummaryValueFmtWrap::new(key, value)))?;
        }
        Ok(())
    }
}
