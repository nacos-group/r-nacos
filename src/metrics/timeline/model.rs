use crate::metrics::metrics_key::MetricsKey;
use crate::metrics::model::{CounterValue, GaugeValue, HistogramValue, SummaryValue};
use crate::metrics::summary::DEFAULT_SUMMARY_BOUNDS;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
pub struct SummaryWrapValue {
    pub(crate) value: SummaryValue,
    pub(crate) rps: f64,
    pub(crate) average: f64,
}

impl SummaryWrapValue {
    pub fn new(value: SummaryValue, diff_ms: u64) -> Self {
        let rps = if diff_ms > 0 {
            value.count as f64 / (diff_ms as f64 / 1000f64)
        } else {
            value.count as f64
        };
        let average = if value.count > 0 {
            value.sum / value.count as f64
        } else {
            0f64
        };
        Self {
            value,
            rps,
            average,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct TimelineValue {
    pub(crate) snapshot: MetricsSnapshot,
    pub(crate) section_gauge: HashMap<MetricsKey, f64>,
    pub(crate) section_summary: HashMap<MetricsKey, SummaryWrapValue>,
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
            let diff_ms = self.snapshot.snapshot_time - last_snapshot.snapshot.snapshot_time;
            for (key, item) in self
                .snapshot
                .diff_histogram(&last_snapshot.snapshot.histogram_data_map)
            {
                let summary_key = MetricsKey::get_summary_from_histogram(&key).unwrap_or(key);
                let mut summary = SummaryValue::new(&DEFAULT_SUMMARY_BOUNDS);
                summary.recalculate_from_histogram(&item);
                self.section_summary
                    .insert(summary_key, SummaryWrapValue::new(summary, diff_ms));
            }
        } else {
            for (key, item) in &self.snapshot.counter_data_map {
                self.section_gauge.insert(key.to_owned(), item.0 as f64);
            }
            let diff_ms = 5000u64;
            for (key, item) in &self.snapshot.histogram_data_map {
                let summary_key =
                    MetricsKey::get_summary_from_histogram(key).unwrap_or(key.to_owned());
                let mut summary = SummaryValue::new(&DEFAULT_SUMMARY_BOUNDS);
                summary.recalculate_from_histogram(&item);
                self.section_summary
                    .insert(summary_key, SummaryWrapValue::new(summary, diff_ms));
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

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineSummary {
    pub bound_keys: Vec<String>,
    pub bounds: Vec<f64>,
    pub rps_data: Vec<f64>,
    pub average_data: Vec<f64>,
    pub count_data: Vec<u64>,
    pub items_data: HashMap<String, Vec<f64>>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineQueryResponse {
    pub last_time: u64,
    pub from_node_id: u64,
    pub time_index: Vec<u64>,
    pub gauge_data: HashMap<String, Vec<f64>>,
    pub summery_data: HashMap<String, TimelineSummary>,
}
