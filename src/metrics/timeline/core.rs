use crate::metrics::metrics_key::MetricsKey;
use crate::metrics::model::SummaryValue;
use crate::metrics::timeline::model::{
    MetricsSnapshot, TimelineQueryParam, TimelineQueryResponse, TimelineValue,
};
use std::collections::{HashMap, LinkedList};

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

    pub fn query(&self, param: TimelineQueryParam) -> TimelineQueryResponse {
        let mut item_list = vec![];
        for item in self.timelines.iter() {
            if item.snapshot.snapshot_time <= param.start_time {
                continue;
            }
            item_list.push(item);
        }
        if item_list.is_empty() {
            return TimelineQueryResponse::default();
        }
        let mut keys = Vec::with_capacity(param.keys.len());
        for str_key in param.keys.iter() {
            if let Some(key) = MetricsKey::of_key(str_key) {
                keys.push(key);
            }
        }
        let mut time_index: Vec<u64> = Vec::with_capacity(item_list.len());
        let mut gauge_data: HashMap<String, Vec<f64>> = HashMap::new();
        let mut summery_keys: HashMap<String, Vec<String>> = HashMap::new();
        for item in item_list {
            time_index.push(item.snapshot.snapshot_time);
            for key in keys.iter() {
                if let Some(v) = item.section_gauge.get(key) {
                    Self::fill_gauge_value(&mut gauge_data, key.get_key(), *v, time_index.len());
                } else if let Some(v) = item.section_summary.get(key) {
                    let str_key = key.get_key();
                    let sub_keys = if let Some(sub_keys) = summery_keys.get(str_key) {
                        sub_keys
                    } else {
                        let sub_keys = Self::build_summary_sub_keys(v, str_key);
                        summery_keys.insert(str_key.to_owned(), sub_keys);
                        summery_keys.get(str_key).unwrap()
                    };
                    for (i, sub_key) in sub_keys.iter().enumerate() {
                        if let Some(sub_v) = v.buckets.get(i) {
                            Self::fill_gauge_value(
                                &mut gauge_data,
                                sub_key,
                                sub_v.0,
                                time_index.len(),
                            );
                        } else {
                            Self::fill_gauge_value(
                                &mut gauge_data,
                                sub_key,
                                0f64,
                                time_index.len(),
                            );
                        }
                    }
                    if v.count > 0 {
                        Self::fill_gauge_value(
                            &mut gauge_data,
                            &format!("{}__average", str_key),
                            v.sum / v.count as f64,
                            time_index.len(),
                        );
                    } else {
                        Self::fill_gauge_value(
                            &mut gauge_data,
                            &format!("{}__average", str_key),
                            0f64,
                            time_index.len(),
                        );
                    }
                }
            }
        }
        let last_time = time_index.last().cloned().unwrap_or_default();
        TimelineQueryResponse {
            last_time,
            from_node_id: 0,
            time_index,
            gauge_data,
            summery_keys,
        }
    }

    fn fill_gauge_value(
        gauge_data: &mut HashMap<String, Vec<f64>>,
        str_key: &str,
        v: f64,
        list_len: usize,
    ) {
        if let Some(list) = gauge_data.get_mut(str_key) {
            list.push(v);
        } else {
            let mut list = Vec::with_capacity(list_len);
            list.push(v);
            gauge_data.insert(str_key.to_string(), list);
        }
    }

    fn build_summary_sub_keys(summary_value: &SummaryValue, prefix_key: &str) -> Vec<String> {
        summary_value
            .bounds
            .iter()
            .map(|e| format!("{}@{}", prefix_key, e))
            .collect()
    }
}

#[derive(Debug, Clone, Default)]
pub struct MetricsTimelineManager {
    minute_timeline_group: TimelineGroup,
    least_timeline_group: TimelineGroup,
}

impl MetricsTimelineManager {
    pub fn new() -> Self {
        Self {
            minute_timeline_group: TimelineGroup::new(360),
            least_timeline_group: TimelineGroup::new(360),
        }
    }

    pub fn add_minute_record(&mut self, snapshot: MetricsSnapshot) {
        self.minute_timeline_group.add_record(snapshot);
    }

    pub fn add_least_record(&mut self, snapshot: MetricsSnapshot) {
        self.minute_timeline_group.add_record(snapshot);
    }

    pub fn query(&self, param: TimelineQueryParam) -> TimelineQueryResponse {
        if param.timeline_group_name == "LEAST" {
            self.least_timeline_group.query(param)
        } else {
            self.minute_timeline_group.query(param)
        }
    }

    pub fn last_minute_record_time(&self) -> u64 {
        self.minute_timeline_group.last_time
    }
}
