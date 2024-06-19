use std::collections::HashMap;
use crate::metrics::CounterItem;
use crate::metrics::metrics_type::MetricsType;



type Key = MetricsType;

#[derive(Default,Debug)]
pub struct CounterManager {
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

