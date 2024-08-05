use crate::metrics::metrics_key::MetricsKey;
use crate::metrics::model::{CounterValue, GaugeValue, HistogramValue, SummaryValue};
use crate::metrics::summary::DEFAULT_SUMMARY_BOUNDS;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum TimelineGroupType {
    Least, //可由配置控制，默认15秒
    Minute,
    Hour,
}

/*
lazy_static::lazy_static! {
    /// 用于有序遍历打印信息
    pub static ref TIMELINE_GROUP_ALL_KEYS: Vec<TimelineGroupType> = vec![
        TimelineGroupType::Least,
        TimelineGroupType::Minute,
        TimelineGroupType::Hour,
    ];
}
*/

impl TimelineGroupType {
    pub fn get_key(&self) -> &'static str {
        match self {
            TimelineGroupType::Least => "LEAST",
            TimelineGroupType::Minute => "MINUTE",
            TimelineGroupType::Hour => "HOUR",
        }
    }

    pub fn get_interval_second(&self) -> u64 {
        match self {
            TimelineGroupType::Least => 0,
            TimelineGroupType::Minute => 60,
            TimelineGroupType::Hour => 3600,
        }
    }

    pub fn get_interval_millis(&self) -> u64 {
        match self {
            TimelineGroupType::Least => 0,
            TimelineGroupType::Minute => 60_000,
            TimelineGroupType::Hour => 3_600_000,
        }
    }

    pub fn from_key(name: &str) -> Option<Self> {
        match name {
            "LEAST" => Some(Self::Least),
            "MINUTE" => Some(Self::Minute),
            "HOUR" => Some(Self::Hour),
            _ => None,
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
pub struct SummaryWrapValue {
    pub(crate) value: SummaryValue,
    pub(crate) rps: f32,
    pub(crate) average: f32,
}

impl SummaryWrapValue {
    pub fn new(value: SummaryValue, diff_ms: u64) -> Self {
        let rps = if diff_ms > 0 {
            value.count as f32 / (diff_ms as f32 / 1000f32)
        } else {
            value.count as f32
        };
        let average = if value.count > 0 {
            value.sum / value.count as f32
        } else {
            0f32
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
    pub(crate) section_gauge: HashMap<MetricsKey, f32>,
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
                self.section_gauge.insert(key.to_owned(), item.0 as f32);
            }
            let diff_ms = self.snapshot.snapshot_time - last_snapshot.snapshot.snapshot_time;
            for (key, item) in self
                .snapshot
                .diff_histogram(&last_snapshot.snapshot.histogram_data_map)
            {
                let summary_key =
                    MetricsKey::get_summary_from_histogram(&key).unwrap_or(key.to_owned());
                let mut summary = SummaryValue::new(&DEFAULT_SUMMARY_BOUNDS);
                summary.recalculate_from_histogram(&item);
                /*
                if item.count > 0 {
                    let mut is_empty = true;
                    for item in &summary.buckets {
                        if item.0 != 0f32 {
                            is_empty = false;
                            break;
                        }
                    }
                    if is_empty {
                        log::warn!(
                            "summary result is error,histogram:{},summary:{}",
                            HistogramValueFmtWrap::new(&key, &item),
                            SummaryValueFmtWrap::new(&summary_key, &summary)
                        );
                    }
                }
                 */
                self.section_summary
                    .insert(summary_key, SummaryWrapValue::new(summary, diff_ms));
            }
        } else {
            for (key, item) in &self.snapshot.counter_data_map {
                self.section_gauge.insert(key.to_owned(), item.0 as f32);
            }
            let diff_ms = 5000u64;
            for (key, item) in &self.snapshot.histogram_data_map {
                let summary_key =
                    MetricsKey::get_summary_from_histogram(key).unwrap_or(key.to_owned());
                let mut summary = SummaryValue::new(&DEFAULT_SUMMARY_BOUNDS);
                summary.recalculate_from_histogram(item);
                self.section_summary
                    .insert(summary_key, SummaryWrapValue::new(summary, diff_ms));
            }
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
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
    pub bounds: Vec<f32>,
    pub rps_data: Vec<f32>,
    pub average_data: Vec<f32>,
    pub count_data: Vec<u64>,
    pub items_data: HashMap<String, Vec<f32>>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineQueryResponse {
    pub last_time: u64,
    pub from_node_id: u64,
    pub time_index: Vec<u64>,
    pub interval_second: u64,
    pub gauge_data: HashMap<String, Vec<f32>>,
    pub summery_data: HashMap<String, TimelineSummary>,
}
