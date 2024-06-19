use std::collections::HashMap;
use crate::metrics::counter::CounterManager;
use crate::metrics::gauge::GaugeManager;
use crate::metrics::metrics_type::MetricsType;

pub struct HistogramManager {

}

pub struct MetricsManager{
    counter_manager: CounterManager,
    gauge_manager: GaugeManager,
}