use crate::metrics::metrics_key::{MetricsKey, ORDER_ALL_KEYS};
use crate::metrics::model::{GaugeValue, GaugeValueFmtWrap};
use bytes::BytesMut;
use std::collections::HashMap;
use std::fmt::Write;

type Key = MetricsKey;

#[derive(Default, Debug)]
pub struct GaugeManager {
    pub(crate) data_map: HashMap<Key, GaugeValue>,
}

impl GaugeManager {
    pub fn increment(&mut self, key: Key, value: f32) {
        if let Some(item) = self.data_map.get_mut(&key) {
            item.increment(value);
        } else {
            self.data_map.insert(key, value.into());
        }
    }

    pub fn decrement(&mut self, key: Key, value: f32) {
        if let Some(item) = self.data_map.get_mut(&key) {
            item.increment(value);
        } else {
            self.data_map.insert(key, 0f32.into());
        }
    }

    pub fn set(&mut self, key: Key, value: f32) {
        if let Some(item) = self.data_map.get_mut(&key) {
            item.set(value);
        } else {
            self.data_map.insert(key, value.into());
        }
    }

    pub fn value(&self, key: &Key) -> Option<f32> {
        self.data_map.get(key).map(|item| item.0.to_owned())
    }

    pub fn print_metrics(&self) {
        //log::info!("-------------- METRICS GAUGE --------------");
        for key in ORDER_ALL_KEYS.iter() {
            if let Some(val) = self.data_map.get(key) {
                log::info!("[metrics_gauge]|{}:{}|", key.get_key(), val.0);
            }
        }
    }

    pub fn export(&mut self, bytes_mut: &mut BytesMut) -> anyhow::Result<()> {
        for (key, value) in self.data_map.iter() {
            bytes_mut.write_str(&format!("{}", &GaugeValueFmtWrap::new(key, value)))?;
        }
        //bytes_mut.write_str("\n")?;
        Ok(())
    }
}
