use std::collections::HashMap;
use crate::metrics::metrics_type::MetricsType;

pub struct CounterManager{
    date_map : HashMap<MetricsType,u64>,
}

pub struct GaugeManager{
    date_map : HashMap<MetricsType,u64>,
}

pub struct HistogramManager {

}

pub struct MetricsManager{
    //counter: CounterManager,

}