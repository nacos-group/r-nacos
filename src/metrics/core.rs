use crate::metrics::counter::CounterManager;
use crate::metrics::gauge::GaugeManager;
use crate::metrics::metrics_type::MetricsType;
use std::collections::HashMap;

pub struct HistogramManager {}

pub struct MetricsManager {
    counter_manager: CounterManager,
    gauge_manager: GaugeManager,
}
