use std::collections::HashMap;
use crate::metrics::metrics_type::MetricsType;

pub struct GaugeManager{
    date_map : HashMap<MetricsType,u64>,
}
