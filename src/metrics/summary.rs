use crate::metrics::metrics_key::MetricsKey;
use crate::metrics::model::{HistogramValue, SummaryValue, SummaryValueFmtWrap};
use bytes::BytesMut;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::Write;

type Key = MetricsKey;

#[derive(Default, Debug)]
pub struct SummaryManager {
    pub(crate) date_map: HashMap<Key, SummaryValue>,
}

impl SummaryManager {
    pub fn init(&mut self, key: Key, bounds: &[f64]) {
        if let Entry::Vacant(e) = self.date_map.entry(key) {
            if let Some(item) = SummaryValue::new(bounds) {
                e.insert(item);
            }
        }
    }

    pub fn recalculate_from_histogram(&mut self, key: &Key, histogram_value: &HistogramValue) {
        if let Some(item) = self.date_map.get_mut(key) {
            item.recalculate_from_histogram(histogram_value)
        }
    }

    pub fn print_metrics(&self) {
        for (key, value) in self.date_map.iter() {
            log::info!("[metrics_summary]|{}:{}|", key.get_key(), value);
        }
    }

    pub fn export(&mut self, bytes_mut: &mut BytesMut) -> anyhow::Result<()> {
        for (key, value) in self.date_map.iter() {
            bytes_mut.write_str(&format!("{}", &SummaryValueFmtWrap::new(key, value)))?;
        }
        Ok(())
    }
}
