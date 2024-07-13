use crate::metrics::metrics_key::MetricsKey;
use crate::metrics::model::{CounterValue, GaugeValue, HistogramValue, SummaryValue};
use crate::metrics::summary::DEFAULT_SUMMARY_BOUNDS;
use std::collections::{HashMap, LinkedList};
//use crate::metrics::timeline::timeline_key::MetricsTimeLineKey;

#[derive(Debug, Default, Clone)]
pub struct TimelineGroup {
    timelines: LinkedList<TimelineValue>,
    pub(crate) limit_count: usize,
    pub(crate) last_time: u64,
}

impl TimelineGroup {
    pub fn new(limit_count: usize) -> Self {
        Self {
            timelines: LinkedList::new(),
            limit_count,
            last_time: 0,
        }
    }

    pub fn add_record(&mut self, snapshot: MetricsSnapshot) {
        self.last_time = snapshot.snapshot_time;
        let record = TimelineValue::new(snapshot, self.timelines.iter().last());
        self.timelines.push_back(record);
        while self.timelines.len() > self.limit_count {
            self.timelines.pop_front();
        }
    }
}

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
    snapshot: MetricsSnapshot,
    section_gauge: HashMap<MetricsKey, f64>,
    section_summary: HashMap<MetricsKey, SummaryValue>,
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
