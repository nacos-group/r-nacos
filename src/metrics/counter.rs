use std::collections::HashMap;
use crate::metrics::metrics_type::MetricsType;


#[derive(Default,Debug)]
pub struct CounterItem(u64);

impl CounterItem {
    pub fn increment(&mut self, value: u64) {
        self.0 += value;
    }

    pub fn absolute(&mut self, value: u64) {
        self.0 = value;
    }

    pub fn value(&self) -> u64 {
        self.0
    }
}

impl From<CounterItem> for u64 {
    fn from(value: CounterItem) -> Self {
        value.0
    }
}

impl From<u64> for CounterItem {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

type Key = MetricsType;

#[derive(Default,Debug)]
pub struct CounterManager{
    date_map : HashMap<Key,CounterItem>,
}

impl CounterManager {
    pub fn increment(&mut self,key:Key,value:u64) {
        if let Some(item) = self.date_map.get_mut(&key) {
            item.increment(value);
        }
        else {
            self.date_map.insert(key,value.into());
        }
    }

    pub fn absolute(&mut self,key:Key,value:u64) {
        if let Some(item) = self.date_map.get_mut(&key) {
            item.absolute(value);
        }
        else {
            self.date_map.insert(key,value.into());
        }
    }
}

