use crate::metrics::counter::CounterManager;
use crate::metrics::gauge::GaugeManager;
use crate::metrics::histogram::HistogramManager;
use crate::metrics::metrics_type::MetricsType;
use actix::prelude::*;
use bean_factory::{bean, BeanFactory, FactoryData, Inject};

#[bean(inject)]
#[derive(Default, Debug)]
pub struct MetricsManager {
    counter_manager: CounterManager,
    gauge_manager: GaugeManager,
    histogram_manager: HistogramManager,
}

impl Actor for MetricsManager {
    type Context = Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        log::info!("MetricsManager started");
    }
}

impl Inject for MetricsManager {
    type Context = Context<Self>;

    fn inject(&mut self, factory_data: FactoryData, factory: BeanFactory, ctx: &mut Self::Context) {
        todo!()
    }
}
