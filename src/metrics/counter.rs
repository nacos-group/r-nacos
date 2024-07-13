use crate::metrics::metrics_key::{MetricsKey, ORDER_ALL_KEYS};
use crate::metrics::model::{CounterValue, CounterValueFmtWrap};
use bytes::BytesMut;
use std::collections::HashMap;
use std::fmt::Write;

type Key = MetricsKey;

#[derive(Default, Debug)]
pub struct CounterManager {
    pub(crate) data_map: HashMap<Key, CounterValue>,
}

impl CounterManager {
    pub fn increment(&mut self, key: Key, value: u64) {
        if let Some(item) = self.data_map.get_mut(&key) {
            item.increment(value);
        } else {
            self.data_map.insert(key, value.into());
        }
    }

    pub fn absolute(&mut self, key: Key, value: u64) {
        if let Some(item) = self.data_map.get_mut(&key) {
            item.absolute(value);
        } else {
            self.data_map.insert(key, value.into());
        }
    }
    pub fn print_metrics(&self) {
        //log::info!("-------------- METRICS COUNTER --------------");
        for key in ORDER_ALL_KEYS.iter() {
            if let Some(val) = self.data_map.get(key) {
                log::info!("[metrics_counter]|{}:{}|", key.get_key(), val.0);
            }
        }
    }

    pub fn export(&mut self, bytes_mut: &mut BytesMut) -> anyhow::Result<()> {
        for (key, value) in self.data_map.iter() {
            bytes_mut.write_str(&format!("{}", &CounterValueFmtWrap::new(key, value)))?;
        }
        //bytes_mut.write_str("\n")?;
        Ok(())
    }
}
