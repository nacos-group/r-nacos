use crate::metrics::metrics_type::MetricsType;
use actix::prelude::*;
use std::fmt::{Display, Formatter};

#[derive(Default, Debug, Clone)]
pub struct CounterItem(pub(crate) u64);

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

#[derive(Default, Debug, Clone)]
pub struct GaugeItem(pub(crate) f64);

impl GaugeItem {
    /// Increments the gauge by the given amount.
    pub fn increment(&mut self, value: f64) {
        self.0 += value;
    }

    /// Decrements the gauge by the given amount.
    pub fn decrement(&mut self, value: f64) {
        self.0 -= value;
    }

    /// Sets the gauge to the given amount.
    pub fn set(&mut self, value: f64) {
        self.0 = value;
    }
}

impl From<GaugeItem> for f64 {
    fn from(value: GaugeItem) -> Self {
        value.0
    }
}

impl From<f64> for GaugeItem {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

#[derive(Default, Debug, Clone)]
pub struct HistogramItem {
    pub(crate) count: u64,
    pub(crate) sum: f64,
    bounds: Vec<f64>,
    buckets: Vec<CounterItem>,
}

impl HistogramItem {
    pub fn new(bounds: &[f64]) -> Option<HistogramItem> {
        if bounds.is_empty() {
            return None;
        }

        let buckets = vec![CounterItem::default(); bounds.len()];

        Some(HistogramItem {
            count: 0,
            bounds: Vec::from(bounds),
            buckets,
            sum: 0.0,
        })
    }

    pub fn sum(&self) -> f64 {
        self.sum
    }

    pub fn count(&self) -> u64 {
        self.count
    }

    pub fn buckets(&self) -> Vec<(f64, u64)> {
        self.bounds
            .iter()
            .cloned()
            .zip(self.buckets.iter().map(|e| e.clone().into()))
            .collect()
    }

    pub fn record(&mut self, sample: f64) {
        self.sum += sample;
        self.count += 1;

        for (idx, bucket) in self.bounds.iter().enumerate() {
            if sample <= *bucket {
                self.buckets[idx].0 += 1;
            }
        }
    }

    pub fn record_many(&mut self, samples: &[f64]) {
        let mut bucketed = vec![0u64; self.buckets.len()];

        let mut sum = 0.0;
        let mut count = 0;
        for sample in samples.into_iter() {
            sum += *sample;
            count += 1;

            for (idx, bucket) in self.bounds.iter().enumerate() {
                if sample <= bucket {
                    bucketed[idx] += 1;
                    break;
                }
            }
        }

        if bucketed.len() >= 2 {
            for idx in 0..(bucketed.len() - 1) {
                bucketed[idx + 1] += bucketed[idx];
            }
        }

        for (idx, local) in bucketed.iter().enumerate() {
            self.buckets[idx].0 += local;
        }
        self.sum += sum;
        self.count += count;
    }
}

impl Display for HistogramItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "total_count={},total_sum={}", self.count, self.sum).ok();
        for (k, v) in self.buckets() {
            write!(f, "|le={},count={}", k, v).ok();
        }
        write!(f, "|le=+Inf,count={}|", self.count)
    }
}

#[derive(Message)]
#[rtype(result = "anyhow::Result<Vec<MetricsItem>>")]
pub struct MetricsQuery;

pub enum MetricsRecord {
    CounterInc(u64),
    Gauge(f64),
    HistogramRecord(f64),
    HistogramRecords(Vec<f64>),
}

impl MetricsRecord {
    pub fn value_str(&self) -> String {
        match &self {
            MetricsRecord::CounterInc(v) => v.to_string(),
            MetricsRecord::Gauge(v) => v.to_string(),
            MetricsRecord::HistogramRecord(v) => v.to_string(),
            MetricsRecord::HistogramRecords(v) => "Vec<[]>".to_string(),
        }
    }
}

pub struct MetricsItem {
    pub metrics_type: MetricsType,
    pub record: MetricsRecord,
}
