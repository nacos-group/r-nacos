use std::collections::HashMap;
use crate::metrics::CounterItem;
use crate::metrics::metrics_type::MetricsType;

type Key = MetricsType;
#[derive(Default,Debug,Clone)]
pub struct HistogramGroup {
    count: u64,
    sum: f64,
    bounds: Vec<f64>,
    buckets: Vec<CounterItem>,
}

impl HistogramGroup {
    pub fn new(bounds: &[f64]) -> Option<HistogramGroup> {
        if bounds.is_empty() {
            return None;
        }

        let buckets = vec![CounterItem::default(); bounds.len()];

        Some(HistogramGroup { count: 0, bounds: Vec::from(bounds), buckets, sum: 0.0 })
    }

    pub fn sum(&self) -> f64 {
        self.sum
    }

    pub fn count(&self) -> u64 {
        self.count
    }

    pub fn buckets(&self) -> Vec<(f64, CounterItem)> {
        self.bounds.iter().cloned().zip(self.buckets.iter().cloned()).collect()
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

    pub fn record_many<'a, S>(&mut self, samples: S)
        where
            S: IntoIterator<Item = &'a f64> + 'a,
    {
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

pub struct HistogramManager {
    date_map : HashMap<Key,HistogramGroup>,
}

impl HistogramManager {

}