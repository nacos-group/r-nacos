use crate::metrics::metrics_key::MetricsKey;
use crate::metrics::model::{MetricsItem, MetricsQuery, MetricsRecord};
use crate::naming::core::NamingActor;
use actix::Handler;

impl Handler<MetricsQuery> for NamingActor {
    type Result = anyhow::Result<Vec<MetricsItem>>;

    fn handle(&mut self, _: MetricsQuery, ctx: &mut Self::Context) -> Self::Result {
        let mut list = vec![];
        list.push(MetricsItem {
            metrics_type: MetricsKey::NamingServiceSize,
            record: MetricsRecord::Gauge(self.service_map.len() as f64),
        });
        list.push(MetricsItem {
            metrics_type: MetricsKey::NamingInstanceSize,
            record: MetricsRecord::Gauge(self.get_instance_size() as f64),
        });
        list.push(MetricsItem {
            metrics_type: MetricsKey::NamingSubscriberListenerKeySize,
            record: MetricsRecord::Gauge(self.subscriber.get_listener_key_size() as f64),
        });
        list.push(MetricsItem {
            metrics_type: MetricsKey::NamingSubscriberListenerValueSize,
            record: MetricsRecord::Gauge(self.subscriber.get_listener_value_size() as f64),
        });
        list.push(MetricsItem {
            metrics_type: MetricsKey::NamingSubscriberClientSize,
            record: MetricsRecord::Gauge(self.subscriber.get_client_size() as f64),
        });
        list.push(MetricsItem {
            metrics_type: MetricsKey::NamingSubscriberClientValueSize,
            record: MetricsRecord::Gauge(self.subscriber.get_client_value_size() as f64),
        });
        list.push(MetricsItem {
            metrics_type: MetricsKey::NamingEmptyServiceSetSize,
            record: MetricsRecord::Gauge(self.empty_service_set.len() as f64),
        });
        list.push(MetricsItem {
            metrics_type: MetricsKey::NamingEmptyServiceSetItemSize,
            record: MetricsRecord::Gauge(self.empty_service_set.item_size() as f64),
        });
        list.push(MetricsItem {
            metrics_type: MetricsKey::NamingInstanceMetaSetSize,
            record: MetricsRecord::Gauge(self.instance_metadate_set.len() as f64),
        });
        list.push(MetricsItem {
            metrics_type: MetricsKey::NamingInstanceMetaSetItemSize,
            record: MetricsRecord::Gauge(self.instance_metadate_set.item_size() as f64),
        });
        list.push(MetricsItem {
            metrics_type: MetricsKey::NamingHealthyTimeoutSetSize,
            record: MetricsRecord::Gauge(self.get_healthy_timeout_set_size() as f64),
        });
        list.push(MetricsItem {
            metrics_type: MetricsKey::NamingHealthyTimeoutSetItemSize,
            record: MetricsRecord::Gauge(self.get_healthy_timeout_set_item_size() as f64),
        });
        list.push(MetricsItem {
            metrics_type: MetricsKey::NamingUnhealthyTimeoutSetSize,
            record: MetricsRecord::Gauge(self.get_unhealthy_timeout_set_size() as f64),
        });
        list.push(MetricsItem {
            metrics_type: MetricsKey::NamingUnhealthyTimeoutSetItemSize,
            record: MetricsRecord::Gauge(self.get_unhealthy_timeout_set_item_size() as f64),
        });
        list.push(MetricsItem {
            metrics_type: MetricsKey::NamingClientInstanceSetKeySize,
            record: MetricsRecord::Gauge(self.client_instance_set.len() as f64),
        });
        list.push(MetricsItem {
            metrics_type: MetricsKey::NamingClientInstanceSetValueSize,
            record: MetricsRecord::Gauge(self.get_client_instance_set_item_size() as f64),
        });
        list.push(MetricsItem {
            metrics_type: MetricsKey::NamingIndexTenantSize,
            record: MetricsRecord::Gauge(self.namespace_index.get_tenant_count() as f64),
        });
        let (group_size, service_size) = self.namespace_index.get_service_count();
        list.push(MetricsItem {
            metrics_type: MetricsKey::NamingIndexGroupSize,
            record: MetricsRecord::Gauge(group_size as f64),
        });
        list.push(MetricsItem {
            metrics_type: MetricsKey::NamingIndexServiceSize,
            record: MetricsRecord::Gauge(service_size as f64),
        });
        Ok(list)
    }
}
