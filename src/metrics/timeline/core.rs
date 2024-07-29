use crate::metrics::metrics_key::MetricsKey;
use crate::metrics::timeline::model::{
    MetricsSnapshot, SummaryWrapValue, TimelineGroupType, TimelineQueryParam,
    TimelineQueryResponse, TimelineSummary, TimelineValue,
};
use std::collections::{HashMap, HashSet, LinkedList};

#[derive(Debug, Default, Clone)]
pub struct TimelineGroup {
    timelines: LinkedList<TimelineValue>,
    pub(crate) limit_count: usize,
    pub(crate) last_time: u64,
    pub(crate) interval_second: u64,
}

impl TimelineGroup {
    pub fn new(limit_count: usize, interval: u64) -> Self {
        Self {
            timelines: LinkedList::new(),
            limit_count,
            last_time: 0,
            interval_second: interval,
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
        if item_list.is_empty() || param.keys.is_empty() {
            return TimelineQueryResponse {
                interval_second: self.interval_second,
                ..Default::default()
            };
        }
        let mut keys = HashSet::with_capacity(param.keys.len());
        for str_key in param.keys.iter() {
            if let Some(key) = MetricsKey::of_key(str_key) {
                keys.insert(key);
            } else {
                log::warn!("MetricsKey::of_key is null,key:{}", &str_key);
            }
        }
        let mut time_index: Vec<u64> = Vec::with_capacity(item_list.len());
        let mut gauge_data: HashMap<String, Vec<f32>> = HashMap::new();
        let mut summery_data: HashMap<String, TimelineSummary> = HashMap::new();
        let index_len = time_index.len();
        for (i, item) in item_list.iter().enumerate() {
            time_index.push(item.snapshot.snapshot_time);
            for key in keys.iter() {
                if let Some(v) = item.section_gauge.get(key) {
                    Self::fill_gauge_value(&mut gauge_data, key.get_key(), *v, index_len);
                } else if let Some(v) = item.section_summary.get(key) {
                    let str_key = key.get_key();
                    let timeline_summary =
                        if let Some(timeline_summary) = summery_data.get_mut(str_key) {
                            timeline_summary
                        } else {
                            let timeline_summary = Self::build_timeline_summary(v, index_len);
                            summery_data.insert(str_key.to_owned(), timeline_summary);
                            summery_data.get_mut(str_key).unwrap()
                        };
                    timeline_summary.rps_data.push(v.rps);
                    timeline_summary.average_data.push(v.average);
                    timeline_summary.count_data.push(v.value.count);
                    for (i, sub_key) in timeline_summary.bound_keys.iter().enumerate() {
                        if let Some(sub_v) = v.value.buckets.get(i) {
                            Self::fill_gauge_value(
                                &mut timeline_summary.items_data,
                                sub_key,
                                sub_v.0,
                                time_index.len(),
                            );
                        } else {
                            Self::fill_gauge_value(
                                &mut timeline_summary.items_data,
                                sub_key,
                                0f32,
                                time_index.len(),
                            );
                        }
                    }
                } else if i == 0 {
                    log::warn!("not found key data,key: {:?}", &key);
                }
            }
        }
        let last_time = time_index.last().cloned().unwrap_or_default();
        TimelineQueryResponse {
            last_time,
            from_node_id: 0,
            time_index,
            interval_second: self.interval_second,
            gauge_data,
            summery_data,
        }
    }

    fn fill_gauge_value(
        gauge_data: &mut HashMap<String, Vec<f32>>,
        str_key: &str,
        v: f32,
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

    fn build_timeline_summary(wrap: &SummaryWrapValue, len: usize) -> TimelineSummary {
        TimelineSummary {
            bounds: wrap.value.bounds.clone(),
            bound_keys: wrap.value.bounds.iter().map(|e| e.to_string()).collect(),
            rps_data: Vec::with_capacity(len),
            average_data: Vec::with_capacity(len),
            count_data: Vec::with_capacity(len),
            items_data: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MetricsTimelineManager {
    least_timeline_group: TimelineGroup,
    minute_timeline_group: TimelineGroup,
    hour_timeline_group: TimelineGroup,
}

impl MetricsTimelineManager {
    pub fn new() -> Self {
        Self {
            least_timeline_group: TimelineGroup::new(180, 15),
            minute_timeline_group: TimelineGroup::new(360, 60),
            hour_timeline_group: TimelineGroup::new(360, 3600),
        }
    }

    pub fn set_least_interval(&mut self, least_interval: u64) {
        self.least_timeline_group.interval_second = least_interval;
    }

    fn get_timeline_group_mut(&mut self, group_type: &TimelineGroupType) -> &mut TimelineGroup {
        match group_type {
            TimelineGroupType::Least => &mut self.least_timeline_group,
            TimelineGroupType::Minute => &mut self.minute_timeline_group,
            TimelineGroupType::Hour => &mut self.hour_timeline_group,
        }
    }

    fn get_timeline_group(&self, group_type: &TimelineGroupType) -> &TimelineGroup {
        match group_type {
            TimelineGroupType::Least => &self.least_timeline_group,
            TimelineGroupType::Minute => &self.minute_timeline_group,
            TimelineGroupType::Hour => &self.hour_timeline_group,
        }
    }

    pub fn add_record(&mut self, group_type: &TimelineGroupType, snapshot: MetricsSnapshot) {
        let time_line_group = self.get_timeline_group_mut(group_type);
        time_line_group.add_record(snapshot);
    }

    pub fn query(&self, param: TimelineQueryParam) -> TimelineQueryResponse {
        if let Some(group_type) = TimelineGroupType::from_key(&param.timeline_group_name) {
            self.get_timeline_group(&group_type).query(param)
        } else {
            self.minute_timeline_group.query(param)
        }
    }

    pub fn get_last_record_time(&self, group_type: &TimelineGroupType) -> u64 {
        self.get_timeline_group(group_type).last_time
    }
}
