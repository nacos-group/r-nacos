use crate::config::core::ConfigActor;
use crate::grpc::bistream_manage::BiStreamManage;
use crate::metrics::counter::CounterManager;
use crate::metrics::gauge::GaugeManager;
use crate::metrics::histogram::HistogramManager;
use crate::metrics::model::{MetricsItem, MetricsQuery, MetricsRecord};
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
    config_actor: Option<Addr<ConfigActor>>,
    bi_stream_manage: Option<Addr<BiStreamManage>>,
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
        config_actor: Option<Addr<ConfigActor>>,
        bi_stream_manage: Option<Addr<BiStreamManage>>,
    ) -> anyhow::Result<Vec<MetricsItem>> {
        let mut list = vec![];
        if let Some(naming_actor) = naming_actor {
            let mut t = naming_actor.send(MetricsQuery).await??;
            list.append(&mut t);
        }
        if let Some(config_actor) = config_actor {
            let mut t = config_actor.send(MetricsQuery).await??;
            list.append(&mut t);
        }
        if let Some(bi_stream_manage) = bi_stream_manage {
            let mut t = bi_stream_manage.send(MetricsQuery).await??;
            list.append(&mut t);
        }
        Ok(list)
    }

    fn update_peek_metrics(&mut self, r: anyhow::Result<Vec<MetricsItem>>) {
        if let Ok(list) = r {
            for item in list {
                match item.record {
                    MetricsRecord::CounterInc(v) => {
                        self.counter_manager.increment(item.metrics_type, v)
                    }
                    MetricsRecord::Gauge(v) => self.gauge_manager.set(item.metrics_type, v),
                    MetricsRecord::HistogramRecord(v) => {
                        self.histogram_manager.record(&item.metrics_type, v)
                    }
                    MetricsRecord::HistogramRecords(batch_value) => self
                        .histogram_manager
                        .record_many(&item.metrics_type, &batch_value),
                }
            }
        }
    }
    fn print_metrics(&self) {
        //log::info!("-------------- log metrics start --------------");
        self.gauge_manager.print_metrics();
        self.counter_manager.print_metrics();
        self.histogram_manager.print_metrics();
    }

    fn query_metrics(&mut self, ctx: &mut Context<Self>) {
        let naming_actor = self.naming_actor.clone();
        let config_actor = self.config_actor.clone();
        let bi_stream_manage = self.bi_stream_manage.clone();
        async move { Self::do_peek_metrics(naming_actor, config_actor, bi_stream_manage).await }
            .into_actor(self)
            .map(|r, act, ctx| {
                //Self::log_metrics(&r);
                act.update_peek_metrics(r);
                act.print_metrics();
                act.hb(ctx);
            })
            .spawn(ctx);
    }
    fn hb(&mut self, ctx: &mut Context<Self>) {
        ctx.run_later(Duration::from_secs(10), |act, ctx| {
            act.query_metrics(ctx);
        });
    }
}

impl Actor for MetricsManager {
    type Context = Context<Self>;
    fn started(&mut self, _ctx: &mut Self::Context) {
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
        self.config_actor = factory_data.get_actor();
        self.bi_stream_manage = factory_data.get_actor();
        self.init(ctx);
    }
}
