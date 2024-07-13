use crate::metrics::metrics_key::MetricsKey;
use crate::metrics::model::{CounterValue, GaugeValue, HistogramValue, SummaryValue};
use crate::metrics::summary::DEFAULT_SUMMARY_BOUNDS;
use std::collections::HashMap;
//use crate::metrics::timeline::timeline_key::MetricsTimeLineKey;

#[derive(Debug, Default, Clone)]
pub struct MetricsSnapshot {
    pub(crate) gauge_data_map: HashMap<MetricsKey, GaugeValue>,
    pub(crate) counter_data_map: HashMap<MetricsKey, CounterValue>,
    pub(crate) histogram_data_map: HashMap<MetricsKey, HistogramValue>,
    pub(crate) snapshot_time: u64,
}

impl MetricsSnapshot {
    pub fn diff_counter(
        &self,
        old: &HashMap<MetricsKey, CounterValue>,
    ) -> HashMap<MetricsKey, CounterValue> {
        let mut data_map = HashMap::new();
        for (key, value) in self.counter_data_map.iter() {
            let v = if let Some(old_value) = old.get(key) {
                value.diff(old_value)
            } else {
                value.to_owned()
            };
            data_map.insert(key.to_owned(), v);
        }
        data_map
    }

    pub fn diff_histogram(
        &self,
        old: &HashMap<MetricsKey, HistogramValue>,
    ) -> HashMap<MetricsKey, HistogramValue> {
        let mut data_map = HashMap::new();
        for (key, value) in self.histogram_data_map.iter() {
            let v = if let Some(old_value) = old.get(key) {
                value.diff(old_value)
            } else {
                value.to_owned()
            };
            data_map.insert(key.to_owned(), v);
        }
        data_map
    }
}

#[derive(Debug, Default, Clone)]
pub struct TimelineValue {
    pub(crate) snapshot: MetricsSnapshot,
    pub(crate) section_gauge: HashMap<MetricsKey, f64>,
    pub(crate) section_summary: HashMap<MetricsKey, SummaryValue>,
}

impl TimelineValue {
    pub fn new(snapshot: MetricsSnapshot, last_snapshot: Option<&TimelineValue>) -> Self {
        let mut s = Self {
            snapshot,
            section_gauge: HashMap::new(),
            section_summary: HashMap::new(),
        };
        s.init(last_snapshot);
        s
    }

    fn init(&mut self, last_snapshot: Option<&TimelineValue>) {
        for (key, item) in &self.snapshot.gauge_data_map {
            self.section_gauge.insert(key.to_owned(), item.0);
        }
        if let Some(last_snapshot) = last_snapshot {
            for (key, item) in &self
                .snapshot
                .diff_counter(&last_snapshot.snapshot.counter_data_map)
            {
                self.section_gauge.insert(key.to_owned(), item.0 as f64);
            }
            for (key, item) in self
                .snapshot
                .diff_histogram(&last_snapshot.snapshot.histogram_data_map)
            {
                let mut summary = SummaryValue::new(&DEFAULT_SUMMARY_BOUNDS);
                summary.recalculate_from_histogram(&item);
                self.section_summary.insert(key, summary);
            }
        } else {
            for (key, item) in &self.snapshot.counter_data_map {
                self.section_gauge.insert(key.to_owned(), item.0 as f64);
            }
            for (key, item) in &self.snapshot.histogram_data_map {
                let mut summary = SummaryValue::new(&DEFAULT_SUMMARY_BOUNDS);
                summary.recalculate_from_histogram(&item);
                self.section_summary.insert(key.to_owned(), summary);
            }
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct TimelineQueryParam {
    pub start_time: u64,
    // LEAST | MINUTE
    pub timeline_group_name: String,
    //pub query_all_key: bool,
    pub keys: Vec<String>,
    pub node_id: u64,
}

#[derive(Debug, Default, Clone)]
pub struct TimelineQueryResponse {
    pub last_time: u64,
    pub from_node_id: u64,
    pub time_index: Vec<u64>,
    pub gauge_data: HashMap<String, Vec<f64>>,
    pub summery_keys: HashMap<String, Vec<String>>,
}
