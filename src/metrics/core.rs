use crate::config::core::ConfigActor;
use crate::grpc::bistream_manage::BiStreamManage;
use crate::metrics::counter::CounterManager;
use crate::metrics::gauge::GaugeManager;
use crate::metrics::histogram::HistogramManager;
use crate::metrics::metrics_key::MetricsKey;
use crate::metrics::model::{MetricsItem, MetricsQuery, MetricsRecord, MetricsRequest};
use crate::naming::core::NamingActor;
use crate::now_millis;
use actix::prelude::*;
use bean_factory::{bean, BeanFactory, FactoryData, Inject};
use std::time::Duration;
use sysinfo::{Pid, System};

#[bean(inject)]
#[derive(Debug)]
pub struct MetricsManager {
    counter_manager: CounterManager,
    gauge_manager: GaugeManager,
    histogram_manager: HistogramManager,
    naming_actor: Option<Addr<NamingActor>>,
    config_actor: Option<Addr<ConfigActor>>,
    bi_stream_manage: Option<Addr<BiStreamManage>>,
    system: System,
    current_process_id: u32,
    start_time_millis: u64,
    //last_load_time: u64,
    total_memory: f64,
}

impl Default for MetricsManager {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsManager {
    pub fn new() -> Self {
        let current_process_id = std::process::id();
        let start_time_millis = now_millis();
        let mut system = System::new();
        system.refresh_memory();
        let total_memory = system.total_memory() as f64 / (1024.0 * 1024.0);
        let mut gauge_manager = GaugeManager::default();
        gauge_manager.set(MetricsKey::SysTotalMemory, total_memory);
        Self {
            counter_manager: Default::default(),
            gauge_manager,
            histogram_manager: Default::default(),
            naming_actor: None,
            config_actor: None,
            bi_stream_manage: None,
            system,
            current_process_id,
            start_time_millis,
            //last_load_time: 0,
            total_memory,
        }
    }
    fn init(&mut self, ctx: &mut Context<Self>) {
        self.init_histogram();
        self.hb(ctx);
    }

    fn init_histogram(&mut self) {
        // 单位毫秒ms
        self.histogram_manager.init(
            MetricsKey::GrpcRequestHandleRtHistogram,
            &[
                0.5f64, 1f64, 3f64, 5f64, 10f64, 25f64, 50f64, 100f64, 300f64, 500f64, 1000f64,
            ],
        );
        // 单位毫秒ms
        self.histogram_manager.init(
            MetricsKey::HttpRequestHandleRtHistogram,
            &[
                0.5f64, 1f64, 3f64, 5f64, 10f64, 25f64, 50f64, 100f64, 300f64, 500f64, 1000f64,
            ],
        );
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
                self.update_item_record(item);
            }
        }
    }

    fn update_item_record(&mut self, item: MetricsItem) {
        match item.record {
            MetricsRecord::CounterInc(v) => self.counter_manager.increment(item.metrics_type, v),
            MetricsRecord::Gauge(v) => self.gauge_manager.set(item.metrics_type, v),
            MetricsRecord::HistogramRecord(v) => {
                self.histogram_manager.record(&item.metrics_type, v)
            }
            MetricsRecord::HistogramRecords(batch_value) => self
                .histogram_manager
                .record_many(&item.metrics_type, &batch_value),
        }
    }

    fn print_metrics(&self) {
        //log::info!("-------------- log metrics start --------------");
        self.gauge_manager.print_metrics();
        self.counter_manager.print_metrics();
        self.histogram_manager.print_metrics();
    }

    fn load_sys_metrics(&mut self) {
        self.system.refresh_all();
        if let Some(process) = self.system.process(Pid::from_u32(self.current_process_id)) {
            let cpu_usage = process.cpu_usage() as f64;
            let rss = process.memory() as f64 / (1024.0 * 1024.0);
            let vms = process.virtual_memory() as f64 / (1024.0 * 1024.0);
            let rss_usage = rss / self.total_memory * 100.0;
            let running_seconds = (now_millis() - self.start_time_millis) / 1000;
            log::info!("[metrics_system]|already running seconds:{}s|cpu_usage: {:.2}%|rss_usage: {:.2}%|rss: {}M|vms: {}M|total_memory: {}M|",running_seconds,&cpu_usage,&rss_usage,&rss,&vms,&self.total_memory);
            self.gauge_manager.set(MetricsKey::AppCpuUsage, cpu_usage);
            self.gauge_manager.set(MetricsKey::AppRssMemory, rss);
            self.gauge_manager.set(MetricsKey::AppVmsMemory, vms);
            self.gauge_manager.set(MetricsKey::AppRssMemory, rss_usage);
        }
    }

    fn load_metrics(&mut self, ctx: &mut Context<Self>) {
        let naming_actor = self.naming_actor.clone();
        let config_actor = self.config_actor.clone();
        let bi_stream_manage = self.bi_stream_manage.clone();
        async move { Self::do_peek_metrics(naming_actor, config_actor, bi_stream_manage).await }
            .into_actor(self)
            .map(|r, act, ctx| {
                //Self::log_metrics(&r);
                act.update_peek_metrics(r);
                act.load_sys_metrics();
                act.print_metrics();
                act.hb(ctx);
            })
            .spawn(ctx);
    }
    fn hb(&mut self, ctx: &mut Context<Self>) {
        ctx.run_later(Duration::from_secs(10), |act, ctx| {
            act.load_metrics(ctx);
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

impl Handler<MetricsRequest> for MetricsManager {
    type Result = anyhow::Result<()>;

    fn handle(&mut self, msg: MetricsRequest, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            MetricsRequest::Record(item) => {
                self.update_item_record(item);
            }
            MetricsRequest::BatchRecord(items) => {
                for item in items {
                    self.update_item_record(item);
                }
            }
        }
        Ok(())
    }
}
