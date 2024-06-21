use crate::metrics::counter::CounterManager;
use crate::metrics::gauge::GaugeManager;
use crate::metrics::histogram::HistogramManager;
use crate::metrics::metrics_type::MetricsType;
use crate::metrics::model::{MetricsItem, MetricsQuery};
use crate::naming::core::NamingActor;
use actix::prelude::*;
use bean_factory::{bean, BeanFactory, FactoryData, Inject};
use std::time::Duration;

#[bean(inject)]
#[derive(Default, Debug)]
pub struct MetricsManager {
    counter_manager: CounterManager,
    gauge_manager: GaugeManager,
    histogram_manager: HistogramManager,
    naming_actor: Option<Addr<NamingActor>>,
}

impl MetricsManager {
    pub fn new() -> Self {
        Self::default()
    }
    fn init(&mut self, ctx: &mut Context<Self>) {
        self.hb(ctx);
    }

    async fn do_peek_metrics(
        naming_actor: Option<Addr<NamingActor>>,
    ) -> anyhow::Result<Vec<MetricsItem>> {
        let mut list = vec![];
        if let Some(naming_actor) = naming_actor {
            let mut t = naming_actor.send(MetricsQuery).await??;
            list.append(&mut t);
        }
        Ok(list)
    }

    fn log_metrics(r: anyhow::Result<Vec<MetricsItem>>) {
        match r {
            Ok(list) => {
                log::info!("log metrics start");
                for item in list {
                    log::info!(
                        "[MetricsManager metrics]|{}:{}|",
                        item.metrics_type.get_key(),
                        item.record.value_str()
                    )
                }
            }
            Err(e) => {
                log::info!("log metrics error,{}", e);
            }
        }
    }

    fn query_metrics(&mut self, ctx: &mut Context<Self>) {
        let naming_actor = self.naming_actor.clone();
        async move { Self::do_peek_metrics(naming_actor).await }
            .into_actor(self)
            .map(|r, act, ctx| {
                Self::log_metrics(r);
                act.hb(ctx);
            })
            .spawn(ctx);
    }
    fn hb(&mut self, ctx: &mut Context<Self>) {
        ctx.run_later(Duration::from_secs(30), |act, ctx| {
            act.query_metrics(ctx);
        });
    }
}

impl Actor for MetricsManager {
    type Context = Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        log::info!("MetricsManager started");
    }
}

impl Inject for MetricsManager {
    type Context = Context<Self>;

    fn inject(
        &mut self,
        factory_data: FactoryData,
        _factory: BeanFactory,
        ctx: &mut Self::Context,
    ) {
        self.naming_actor = factory_data.get_actor();
        self.init(ctx);
    }
}
