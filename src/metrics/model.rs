use crate::metrics::metrics_key::MetricsKey;
use crate::metrics::timeline::model::{TimelineQueryParam, TimelineQueryResponse};
use actix::prelude::*;
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};

pub enum MetricsType {
    Counter,
    Gauge,
    Histogram,
    Summary,
}

impl MetricsType {
    pub fn get_name(&self) -> &'static str {
        match &self {
            MetricsType::Counter => "counter",
            MetricsType::Gauge => "gauge",
            MetricsType::Histogram => "histogram",
            MetricsType::Summary => "summary",
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct CounterValue(pub(crate) u64);

/*
pub trait MetricsFmt: Sized {
    fn fmt(&self,metrics_key: &MetricsKey,metrics_type:&MetricsType) -> anyhow::Result<String>;
}
 */

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

    pub fn diff(&self, old_value: &Self) -> Self {
        let v = self.0.saturating_sub(old_value.0);
        CounterValue(v)
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

pub(crate) struct CounterValueFmtWrap<'a> {
    metrics_key: &'a MetricsKey,
    value: &'a CounterValue,
}

impl<'a> CounterValueFmtWrap<'a> {
    pub(crate) fn new(metrics_key: &'a MetricsKey, value: &'a CounterValue) -> Self {
        Self { metrics_key, value }
    }
}

impl Display for CounterValueFmtWrap<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let key_name = self.metrics_key.get_key();
        writeln!(
            f,
            "# HELP {} {}\n# TYPE {} {}\n{} {}",
            key_name,
            self.metrics_key.get_describe(),
            key_name,
            MetricsType::Counter.get_name(),
            self.metrics_key.get_key_with_label(),
            self.value.0
        )
    }
}

#[derive(Default, Debug, Clone)]
pub struct GaugeValue(pub(crate) f32);

impl GaugeValue {
    /// Increments the gauge by the given amount.
    pub fn increment(&mut self, value: f32) {
        self.0 += value;
    }

    /// Decrements the gauge by the given amount.
    pub fn decrement(&mut self, value: f32) {
        self.0 -= value;
    }

    /// Sets the gauge to the given amount.
    pub fn set(&mut self, value: f32) {
        self.0 = value;
    }
}

impl From<GaugeValue> for f32 {
    fn from(value: GaugeValue) -> Self {
        value.0
    }
}

impl From<f32> for GaugeValue {
    fn from(value: f32) -> Self {
        Self(value)
    }
}

pub(crate) struct GaugeValueFmtWrap<'a> {
    metrics_key: &'a MetricsKey,
    value: &'a GaugeValue,
}

impl<'a> GaugeValueFmtWrap<'a> {
    pub(crate) fn new(metrics_key: &'a MetricsKey, value: &'a GaugeValue) -> Self {
        Self { metrics_key, value }
    }
}

impl Display for GaugeValueFmtWrap<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let key_name = self.metrics_key.get_key();
        writeln!(
            f,
            "# HELP {} {}\n# TYPE {} {}\n{} {:.3}",
            key_name,
            self.metrics_key.get_describe(),
            key_name,
            MetricsType::Gauge.get_name(),
            self.metrics_key.get_key_with_label(),
            self.value.0
        )
    }
}

#[derive(Default, Debug, Clone)]
pub struct HistogramValue {
    pub(crate) count: u64,
    pub(crate) sum: f32,
    bounds: Vec<f32>,
    buckets: Vec<CounterValue>,
}

impl HistogramValue {
    pub fn new(bounds: &[f32]) -> Option<HistogramValue> {
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

    pub fn sum(&self) -> f32 {
        self.sum
    }

    pub fn count(&self) -> u64 {
        self.count
    }

    pub fn buckets(&self) -> Vec<(f32, u64)> {
        self.bounds
            .iter()
            .cloned()
            .zip(self.buckets.iter().map(|e| e.clone().into()))
            .collect()
    }

    ///
    /// 计算百分位的近似值
    ///
    pub fn approximate_quantile(&self, q: f32) -> f32 {
        if self.count == 0 || q <= 0f32 || self.bounds.is_empty() {
            return 0f32;
        }
        let buckets = self.buckets();
        let q_value = (q * (self.count as f32)) as u64;
        if q_value == 0 {
            return 0f32;
        }
        let mut p_bound = 0f32;
        let mut p_sum_count = 0u64;
        let mut bound = 0f32;
        let mut sum_count = 0u64;
        let mut i = 0;
        while i < buckets.len() {
            let (b, v) = buckets.get(i).unwrap();
            b.clone_into(&mut bound);
            v.clone_into(&mut sum_count);
            match q_value.cmp(&sum_count) {
                Ordering::Less => {
                    return p_bound
                        + ((q_value - p_sum_count) as f32 / (sum_count - p_sum_count) as f32)
                            * (bound - p_bound);
                }
                Ordering::Equal => {
                    return b.to_owned();
                }
                Ordering::Greater => {
                    if self.count == sum_count {
                        return bound;
                    }
                    i += 1;
                    if i < buckets.len() {
                        p_bound = bound;
                        p_sum_count = sum_count;
                    }
                }
            }
        }
        let interval_bound = if i > 1 {
            (bound - p_bound) * 2f32
        } else {
            bound
        };
        bound + ((q_value - sum_count) as f32 / (self.count - sum_count) as f32) * interval_bound
    }

    pub fn record(&mut self, sample: f32) {
        self.sum += sample;
        self.count += 1;

        for (idx, bucket) in self.bounds.iter().enumerate() {
            if sample <= *bucket {
                self.buckets[idx].0 += 1;
            }
        }
    }

    pub fn record_many(&mut self, samples: &[f32]) {
        let mut bucketed = vec![0u64; self.buckets.len()];

        let mut sum = 0.0;
        let mut count = 0;
        for sample in samples {
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

    pub fn diff(&self, old_value: &Self) -> Self {
        if self.bounds.len() != old_value.bounds.len() {
            log::warn!("HistogramValue diff,self.bounds.len() != old_value.bounds.len()");
            return self.to_owned();
        }
        if self.buckets.len() != old_value.buckets.len() {
            log::warn!("HistogramValue diff,self.buckets.len() != old_value.buckets.len()");
            return self.to_owned();
        }
        let mut buckets = Vec::with_capacity(self.buckets.len());
        for (i, v) in self.buckets.iter().enumerate() {
            let old_v = old_value.buckets.get(i).unwrap();
            buckets.push(v.diff(old_v));
        }
        HistogramValue {
            count: self.count - old_value.count,
            sum: self.sum - old_value.sum,
            bounds: self.bounds.clone(),
            buckets,
        }
    }
}

impl Display for HistogramValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "total_count={},total_sum={:.2}", self.count, self.sum).ok();
        for (k, v) in self.buckets() {
            write!(f, "|le={:.1},count={}", k, v).ok();
        }
        write!(f, "|le=+Inf,count={}|", self.count)
    }
}

pub(crate) struct HistogramValueFmtWrap<'a> {
    metrics_key: &'a MetricsKey,
    value: &'a HistogramValue,
}

impl<'a> HistogramValueFmtWrap<'a> {
    pub(crate) fn new(metrics_key: &'a MetricsKey, value: &'a HistogramValue) -> Self {
        Self { metrics_key, value }
    }
}

impl Display for HistogramValueFmtWrap<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let key_name = self.metrics_key.get_key();
        writeln!(
            f,
            "# HELP {} {}\n# TYPE {} {}",
            key_name,
            self.metrics_key.get_describe(),
            key_name,
            MetricsType::Histogram.get_name(),
        )
        .ok();
        for (k, v) in self.value.buckets() {
            writeln!(f, "{}_bucket{{le=\"{}\"}} {}", key_name, k, v).ok();
        }
        writeln!(f, "{}_bucket{{le=\"+Inf\"}} {}", key_name, self.value.count).ok();
        writeln!(f, "{}_sum {:.3}", key_name, self.value.sum).ok();
        writeln!(f, "{}_count {}", key_name, self.value.count).ok();
        Ok(())
    }
}

#[derive(Default, Debug, Clone)]
pub struct SummaryValue {
    pub(crate) count: u64,
    pub(crate) sum: f32,
    pub(crate) bounds: Vec<f32>,
    pub(crate) buckets: Vec<GaugeValue>,
}

impl SummaryValue {
    pub fn new(bounds: &[f32]) -> Self {
        let buckets = vec![GaugeValue::default(); bounds.len()];

        SummaryValue {
            count: 0,
            bounds: Vec::from(bounds),
            buckets,
            sum: 0.0,
        }
    }

    /// 每次查询前通过histogram重新计算summary
    pub fn recalculate_from_histogram(&mut self, histogram_value: &HistogramValue) {
        for (idx, bucket) in self.bounds.iter().enumerate() {
            let v = histogram_value.approximate_quantile(*bucket);
            self.buckets[idx].0 = v;
        }
        self.sum = histogram_value.sum;
        self.count = histogram_value.count;
    }

    pub fn buckets(&self) -> Vec<(f32, f32)> {
        self.bounds
            .iter()
            .cloned()
            .zip(self.buckets.iter().map(|e| e.clone().into()))
            .collect()
    }
}

impl Display for SummaryValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "total_count={},total_sum={:.2}", self.count, self.sum).ok();
        for (k, v) in self.buckets() {
            write!(f, "|quantile={:.2}%,value={:.6}", k * 100f32, v).ok();
        }
        Ok(())
    }
}

pub(crate) struct SummaryValueFmtWrap<'a> {
    metrics_key: &'a MetricsKey,
    value: &'a SummaryValue,
}

impl<'a> SummaryValueFmtWrap<'a> {
    pub(crate) fn new(metrics_key: &'a MetricsKey, value: &'a SummaryValue) -> Self {
        Self { metrics_key, value }
    }
}

impl Display for SummaryValueFmtWrap<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let key_name = self.metrics_key.get_key();
        writeln!(
            f,
            "# HELP {} {}\n# TYPE {} {}",
            key_name,
            self.metrics_key.get_describe(),
            key_name,
            MetricsType::Summary.get_name(),
        )
        .ok();
        for (k, v) in self.value.buckets() {
            writeln!(f, "{}{{quantile=\"{}\"}} {:.6}", key_name, k, v).ok();
        }
        writeln!(f, "{}_sum {:.3}", key_name, self.value.sum).ok();
        writeln!(f, "{}_count {}", key_name, self.value.count).ok();
        Ok(())
    }
}

#[derive(Message)]
#[rtype(result = "anyhow::Result<Vec<MetricsItem>>")]
pub struct MetricsQuery;

#[derive(Message, Clone, Debug)]
#[rtype(result = "anyhow::Result<MetricsResponse>")]
pub enum MetricsRequest {
    Record(MetricsItem),
    BatchRecord(Vec<MetricsItem>),
    TimelineQuery(TimelineQueryParam),
    Export,
}

#[derive(Clone, Debug)]
pub enum MetricsResponse {
    None,
    ExportInfo(String),
    TimelineResponse(TimelineQueryResponse),
}

#[derive(Clone, Debug)]
pub enum MetricsRecord {
    CounterInc(u64),
    Gauge(f32),
    HistogramRecord(f32),
    HistogramRecords(Vec<f32>),
}

#[derive(Clone, Debug)]
pub struct MetricsItem {
    pub metrics_type: MetricsKey,
    pub record: MetricsRecord,
}

impl MetricsItem {
    pub fn new(metrics_type: MetricsKey, record: MetricsRecord) -> Self {
        Self {
            metrics_type,
            record,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::metrics::model::{HistogramValue, SummaryValue};

    #[test]
    fn test_recalculate_from_histogram() {
        let mut histogram_value = HistogramValue::new(&[
            0.25f32, 0.5f32, 1f32, 3f32, 5f32, 10f32, 25f32, 50f32, 100f32, 300f32, 500f32,
        ])
        .unwrap();
        for _ in 0..100 {
            histogram_value.record(0.02f32);
        }
        for _ in 0..100 {
            histogram_value.record(0.04f32);
        }
        for _ in 0..1000 {
            histogram_value.record(0.08f32);
        }
        for _ in 0..100 {
            histogram_value.record(1.6f32);
        }
        for _ in 0..100 {
            histogram_value.record(2.6f32);
        }
        for _ in 0..100 {
            histogram_value.record(7.0f32);
        }
        for _ in 0..50 {
            histogram_value.record(12.0f32);
        }
        for _ in 0..20 {
            histogram_value.record(120.0f32);
        }
        println!("histogram_value|{}", &histogram_value);
        let mut summary_value =
            SummaryValue::new(&[0.5f32, 0.6f32, 0.7f32, 0.8f32, 0.9f32, 0.95f32, 0.9999f32]);
        //println!("summary_value01|{}",&summary_value);
        summary_value.recalculate_from_histogram(&histogram_value);
        println!("summary_value|{}", &summary_value);
    }
}
