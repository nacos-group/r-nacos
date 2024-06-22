use crate::metrics::metrics_key::MetricsKey;
use actix::prelude::*;
use std::fmt::{Display, Formatter};

pub enum MetricsType {
    Counter,
    Gauge,
    Histogram,
}

#[derive(Default, Debug, Clone)]
pub struct CounterValue(pub(crate) u64);

impl CounterValue {
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

impl From<CounterValue> for u64 {
    fn from(value: CounterValue) -> Self {
        value.0
    }
}

impl From<u64> for CounterValue {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Default, Debug, Clone)]
pub struct GaugeValue(pub(crate) f64);

impl GaugeValue {
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

impl From<GaugeValue> for f64 {
    fn from(value: GaugeValue) -> Self {
        value.0
    }
}

impl From<f64> for GaugeValue {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

#[derive(Default, Debug, Clone)]
pub struct HistogramValue {
    pub(crate) count: u64,
    pub(crate) sum: f64,
    bounds: Vec<f64>,
    buckets: Vec<CounterValue>,
}

impl HistogramValue {
    pub fn new(bounds: &[f64]) -> Option<HistogramValue> {
        if bounds.is_empty() {
            return None;
        }

        let buckets = vec![CounterValue::default(); bounds.len()];

        Some(HistogramValue {
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

impl Display for HistogramValue {
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

pub struct MetricsItem {
    pub metrics_type: MetricsKey,
    pub record: MetricsRecord,
}
