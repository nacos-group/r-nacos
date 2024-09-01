use crate::common::AppSysConfig;
use crate::config::core::ConfigActor;
use crate::grpc::bistream_manage::BiStreamManage;
use crate::metrics::counter::CounterManager;
use crate::metrics::gauge::GaugeManager;
use crate::metrics::histogram::HistogramManager;
use crate::metrics::metrics_key::MetricsKey;
use crate::metrics::model::{
    MetricsItem, MetricsQuery, MetricsRecord, MetricsRequest, MetricsResponse,
};
use crate::metrics::summary::SummaryManager;
use crate::metrics::timeline::core::MetricsTimelineManager;
use crate::metrics::timeline::model::{MetricsSnapshot, TimelineGroupType};
use crate::naming::core::NamingActor;
use crate::now_millis;
use actix::prelude::*;
use bean_factory::{bean, BeanFactory, FactoryData, Inject};
use bytes::BytesMut;
use std::sync::Arc;
use std::time::Duration;
use sysinfo::{Pid, System};

#[bean(inject)]
#[derive(Debug)]
pub struct MetricsManager {
    counter_manager: CounterManager,
    gauge_manager: GaugeManager,
    histogram_manager: HistogramManager,
    summary_manager: SummaryManager,
    summary_key_config: Vec<(MetricsKey, MetricsKey)>,
    naming_actor: Option<Addr<NamingActor>>,
    config_actor: Option<Addr<ConfigActor>>,
    bi_stream_manage: Option<Addr<BiStreamManage>>,
    metrics_timeline_manager: MetricsTimelineManager,
    system: System,
    current_process_id: u32,
    start_time_millis: u64,
    total_memory: f32,
    last_collect_time: u64,
    last_log_time: u64,
    app_sys_config: Arc<AppSysConfig>,
}

impl MetricsManager {
    pub fn new(app_sys_config: Arc<AppSysConfig>) -> Self {
        let current_process_id = std::process::id();
        let start_time_millis = now_millis();
        let mut system = System::new();
        system.refresh_memory();
        let total_memory = system.total_memory() as f32 / (1024.0 * 1024.0);
        let mut gauge_manager = GaugeManager::default();
        gauge_manager.set(MetricsKey::SysTotalMemory, total_memory);
        Self {
            counter_manager: Default::default(),
            gauge_manager,
            histogram_manager: Default::default(),
            summary_manager: Default::default(),
            summary_key_config: Default::default(),
            naming_actor: None,
            config_actor: None,
            bi_stream_manage: None,
            metrics_timeline_manager: MetricsTimelineManager::new(),
            system,
            current_process_id,
            start_time_millis,
            total_memory,
            last_collect_time: 0,
            last_log_time: 0,
            app_sys_config,
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
                0.25f32, 0.5f32, 1f32, 3f32, 5f32, 10f32, 25f32, 50f32, 100f32, 300f32, 500f32,
            ],
        );
        self.summary_manager.init(
            MetricsKey::GrpcRequestHandleRtSummary,
            &[0.5f32, 0.6f32, 0.7f32, 0.8f32, 0.9f32, 0.95f32, 1f32],
        );
        // 单位毫秒ms
        self.histogram_manager.init(
            MetricsKey::HttpRequestHandleRtHistogram,
            &[
                0.25f32, 0.5f32, 1f32, 3f32, 5f32, 10f32, 25f32, 50f32, 100f32, 300f32, 500f32,
            ],
        );
        self.summary_manager.init(
            MetricsKey::HttpRequestHandleRtSummary,
            &[0.5f32, 0.6f32, 0.7f32, 0.8f32, 0.9f32, 0.95f32, 1f32],
        );

        //summary from histogram
        self.summary_key_config.push((
            MetricsKey::HttpRequestHandleRtSummary,
            MetricsKey::HttpRequestHandleRtHistogram,
        ));
        self.summary_key_config.push((
            MetricsKey::GrpcRequestHandleRtSummary,
            MetricsKey::GrpcRequestHandleRtHistogram,
        ));
    }

    fn reset_summary(&mut self) {
        for (summary_key, histogram_key) in &self.summary_key_config {
            if let Some(histogram_value) = self.histogram_manager.get_value(histogram_key) {
                self.summary_manager
                    .recalculate_from_histogram(summary_key, histogram_value);
            }
        }
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

    fn print_metrics(&mut self) {
        //不打印监控指标
        if !self.app_sys_config.metrics_log_enable {
            return;
        }
        let now = now_millis();
        if now - self.last_log_time < (self.app_sys_config.metrics_log_interval_second - 1) * 1000 {
            return;
        }
        //log::info!("-------------- log metrics start --------------");
        self.print_sys_metrics();
        self.gauge_manager.print_metrics();
        self.counter_manager.print_metrics();
        self.histogram_manager.print_metrics();
        self.summary_manager.print_metrics();
        self.last_log_time = now;
    }

    fn load_sys_metrics(&mut self) {
        self.system.refresh_all();
        if let Some(process) = self.system.process(Pid::from_u32(self.current_process_id)) {
            let cpu_usage = process.cpu_usage();
            let rss = process.memory() as f32 / (1024.0 * 1024.0);
            let vms = process.virtual_memory() as f32 / (1024.0 * 1024.0);
            let rss_usage = rss / self.total_memory * 100.0;
            let running_seconds = (now_millis() - self.start_time_millis) / 1000;
            //log::info!("[metrics_system]|already running seconds: {}s|cpu_usage: {:.2}%|rss_usage: {:.2}%|rss: {:.2}M|vms: {:.2}M|total_memory: {:.2}M|",running_seconds,&cpu_usage,&rss_usage,&rss,&vms,&self.total_memory);
            self.gauge_manager
                .set(MetricsKey::ProcessStartTimeSeconds, running_seconds as f32);
            self.gauge_manager.set(MetricsKey::AppCpuUsage, cpu_usage);
            self.gauge_manager.set(MetricsKey::AppRssMemory, rss);
            self.gauge_manager.set(MetricsKey::AppVmsMemory, vms);
            self.gauge_manager
                .set(MetricsKey::AppMemoryUsage, rss_usage);
        }
        self.last_collect_time = now_millis();
    }

    fn print_sys_metrics(&self) {
        let cpu_usage = self
            .gauge_manager
            .value(&MetricsKey::AppCpuUsage)
            .unwrap_or_default();
        let rss = self
            .gauge_manager
            .value(&MetricsKey::AppRssMemory)
            .unwrap_or_default();
        let vms = self
            .gauge_manager
            .value(&MetricsKey::AppVmsMemory)
            .unwrap_or_default();
        let rss_usage = self
            .gauge_manager
            .value(&MetricsKey::AppMemoryUsage)
            .unwrap_or_default();
        let running_seconds = self
            .gauge_manager
            .value(&MetricsKey::ProcessStartTimeSeconds)
            .unwrap_or_default();
        log::info!("[metrics_system]|already running seconds: {}s|cpu_usage: {:.2}%|rss_usage: {:.2}%|rss: {:.2}M|vms: {:.2}M|total_memory: {:.2}M|",running_seconds,&cpu_usage,&rss_usage,&rss,&vms,&self.total_memory);
    }

    fn after_peek_metrics(&mut self) {
        self.reset_summary();
        self.load_sys_metrics();
        self.print_metrics();
        let now = now_millis();
        self.record_timeline_snapshot(now, TimelineGroupType::Least);
        self.record_timeline_snapshot(now, TimelineGroupType::Minute);
        self.record_timeline_snapshot(now, TimelineGroupType::Hour);
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
                act.after_peek_metrics();
                act.hb(ctx);
            })
            .spawn(ctx);
    }

    fn build_snapshot(&self, now_ms: u64) -> MetricsSnapshot {
        MetricsSnapshot {
            gauge_data_map: self.gauge_manager.data_map.clone(),
            counter_data_map: self.counter_manager.data_map.clone(),
            histogram_data_map: self.histogram_manager.data_map.clone(),
            snapshot_time: now_ms,
        }
    }

    fn record_timeline_snapshot(&mut self, now_ms: u64, group_type: TimelineGroupType) {
        //增加200ms超过间隔后，增加记录
        if now_ms + 200 - group_type.get_interval_millis()
            > self
                .metrics_timeline_manager
                .get_last_record_time(&group_type)
        {
            self.metrics_timeline_manager
                .add_record(&group_type, self.build_snapshot(now_ms));
        }
    }

    fn hb(&mut self, ctx: &mut Context<Self>) {
        ctx.run_later(
            Duration::from_secs(self.app_sys_config.metrics_collect_interval_second),
            |act, ctx| {
                act.load_metrics(ctx);
            },
        );
    }

    fn export(&mut self) -> anyhow::Result<String> {
        let mut bytes_mut = BytesMut::new();
        self.counter_manager.export(&mut bytes_mut)?;
        self.gauge_manager.export(&mut bytes_mut)?;
        self.histogram_manager.export(&mut bytes_mut)?;
        self.reset_summary();
        self.summary_manager.export(&mut bytes_mut)?;
        Ok(String::from_utf8(bytes_mut.to_vec())?)
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
        self.metrics_timeline_manager
            .set_least_interval(self.app_sys_config.metrics_collect_interval_second);
        if self.app_sys_config.metrics_enable {
            log::info!(
                "metrics enable! log_interval: {}s",
                self.app_sys_config.metrics_log_interval_second
            );
            self.init(ctx);
        } else {
            log::info!("metrics disable!");
        }
    }
}

impl Handler<MetricsRequest> for MetricsManager {
    type Result = anyhow::Result<MetricsResponse>;

    fn handle(&mut self, msg: MetricsRequest, _ctx: &mut Self::Context) -> Self::Result {
        if !self.app_sys_config.metrics_enable {
            return Ok(MetricsResponse::None);
        }
        match msg {
            MetricsRequest::Record(item) => {
                self.update_item_record(item);
                Ok(MetricsResponse::None)
            }
            MetricsRequest::BatchRecord(items) => {
                for item in items {
                    self.update_item_record(item);
                }
                Ok(MetricsResponse::None)
            }
            MetricsRequest::Export => {
                let v = self.export()?;
                Ok(MetricsResponse::ExportInfo(v))
            }
            MetricsRequest::TimelineQuery(param) => {
                let response = self.metrics_timeline_manager.query(param);
                Ok(MetricsResponse::TimelineResponse(response))
            }
        }
    }
}
