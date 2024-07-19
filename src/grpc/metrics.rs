use crate::grpc::bistream_manage::BiStreamManage;
use crate::metrics::metrics_key::MetricsKey;
use crate::metrics::model::{MetricsItem, MetricsQuery, MetricsRecord};
use actix::prelude::*;

impl Handler<MetricsQuery> for BiStreamManage {
    type Result = anyhow::Result<Vec<MetricsItem>>;

    fn handle(&mut self, _msg: MetricsQuery, _ctx: &mut Self::Context) -> Self::Result {
        let list = vec![
            MetricsItem {
                metrics_type: MetricsKey::GrpcConnSize,
                record: MetricsRecord::Gauge(self.conn_cache.len() as f32),
            },
            MetricsItem {
                metrics_type: MetricsKey::GrpcConnActiveTimeoutSetItemSize,
                record: MetricsRecord::Gauge(self.active_time_set.item_size() as f32),
            },
            MetricsItem {
                metrics_type: MetricsKey::GrpcConnResponseTimeoutSetItemSize,
                record: MetricsRecord::Gauge(self.response_time_set.item_size() as f32),
            },
        ];
        Ok(list)
    }
}
