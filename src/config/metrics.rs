use crate::config::core::ConfigActor;
use crate::metrics::metrics_key::MetricsKey;
use crate::metrics::model::{MetricsItem, MetricsQuery, MetricsRecord};
use actix::prelude::*;

impl Handler<MetricsQuery> for ConfigActor {
    type Result = anyhow::Result<Vec<MetricsItem>>;

    fn handle(&mut self, _msg: MetricsQuery, _ctx: &mut Self::Context) -> Self::Result {
        let list = vec![
            MetricsItem {
                metrics_type: MetricsKey::ConfigDataSize,
                record: MetricsRecord::Gauge(self.cache.len() as f64),
            },
            MetricsItem {
                metrics_type: MetricsKey::ConfigListenerClientSize,
                record: MetricsRecord::Gauge(self.listener.get_listener_client_size() as f64),
            },
            MetricsItem {
                metrics_type: MetricsKey::ConfigListenerKeySize,
                record: MetricsRecord::Gauge(self.listener.get_listener_key_size() as f64),
            },
            MetricsItem {
                metrics_type: MetricsKey::ConfigSubscriberListenerKeySize,
                record: MetricsRecord::Gauge(self.subscriber.get_listener_key_size() as f64),
            },
            MetricsItem {
                metrics_type: MetricsKey::ConfigSubscriberListenerValueSize,
                record: MetricsRecord::Gauge(self.subscriber.get_listener_key_size() as f64),
            },
            MetricsItem {
                metrics_type: MetricsKey::ConfigSubscriberClientSize,
                record: MetricsRecord::Gauge(self.subscriber.get_client_size() as f64),
            },
            MetricsItem {
                metrics_type: MetricsKey::ConfigSubscriberClientValueSize,
                record: MetricsRecord::Gauge(self.subscriber.get_client_size() as f64),
            },
            MetricsItem {
                metrics_type: MetricsKey::ConfigIndexTenantSize,
                record: MetricsRecord::Gauge(self.tenant_index.get_tenant_count() as f64),
            },
            MetricsItem {
                metrics_type: MetricsKey::ConfigIndexConfigSize,
                record: MetricsRecord::Gauge(self.tenant_index.get_config_count().1 as f64),
            },
        ];
        Ok(list)
    }
}
