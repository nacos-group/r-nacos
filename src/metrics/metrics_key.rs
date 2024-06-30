//use crate::metrics::model::MetricsType;
use lazy_static::lazy_static;

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum MetricsKey {
    //app
    SysTotalMemory,
    AppRssMemory,
    AppVmsMemory,
    AppMemoryUsage,
    AppCpuUsage,
    //config
    ConfigDataSize,
    ConfigListenerClientSize,
    ConfigListenerKeySize,
    ConfigSubscriberListenerKeySize,
    ConfigSubscriberListenerValueSize,
    ConfigSubscriberClientSize,
    ConfigSubscriberClientValueSize,
    ConfigIndexTenantSize,
    ConfigIndexConfigSize,
    //naming
    NamingServiceSize,
    NamingInstanceSize,
    NamingSubscriberListenerKeySize,
    NamingSubscriberListenerValueSize,
    NamingSubscriberClientSize,
    NamingSubscriberClientValueSize,
    NamingEmptyServiceSetSize,
    NamingEmptyServiceSetItemSize,
    NamingInstanceMetaSetSize,
    NamingInstanceMetaSetItemSize,
    NamingHealthyTimeoutSetSize,
    NamingHealthyTimeoutSetItemSize,
    NamingUnhealthyTimeoutSetSize,
    NamingUnhealthyTimeoutSetItemSize,
    NamingClientInstanceSetKeySize,
    NamingClientInstanceSetValueSize,
    NamingIndexTenantSize,
    NamingIndexGroupSize,
    NamingIndexServiceSize,
    //grpc
    GrpcConnSize,
    GrpcConnActiveTimeoutSetItemSize,
    GrpcConnResponseTimeoutSetItemSize,
    //grpc request
    GrpcRequestHandleRtHistogram,
    GrpcRequestTotalCount,
    //http api request
    HttpRequestHandleRtHistogram,
    HttpRequestTotalCount,
}

lazy_static! {
    /// 用于有序遍历打印信息
    pub static ref ORDER_ALL_KEYS: Vec<MetricsKey> = vec![
        //app
        /*
        MetricsKey::SysTotalMemory,
        MetricsKey::AppRssMemory,
        MetricsKey::AppVmsMemory,
        MetricsKey::AppMemoryUsage,
        MetricsKey::AppCpuUsage,
         */
        //config
        MetricsKey::ConfigDataSize,
        MetricsKey::ConfigListenerClientSize,
        MetricsKey::ConfigListenerKeySize,
        MetricsKey::ConfigSubscriberListenerKeySize,
        MetricsKey::ConfigSubscriberListenerValueSize,
        MetricsKey::ConfigSubscriberClientSize,
        MetricsKey::ConfigSubscriberClientValueSize,
        MetricsKey::ConfigIndexTenantSize,
        MetricsKey::ConfigIndexConfigSize,
        //naming
        MetricsKey::NamingServiceSize,
        MetricsKey::NamingInstanceSize,
        MetricsKey::NamingSubscriberListenerKeySize,
        MetricsKey::NamingSubscriberListenerValueSize,
        MetricsKey::NamingSubscriberClientSize,
        MetricsKey::NamingSubscriberClientValueSize,
        MetricsKey::NamingEmptyServiceSetSize,
        MetricsKey::NamingEmptyServiceSetItemSize,
        MetricsKey::NamingInstanceMetaSetSize,
        MetricsKey::NamingInstanceMetaSetItemSize,
        MetricsKey::NamingHealthyTimeoutSetSize,
        MetricsKey::NamingHealthyTimeoutSetItemSize,
        MetricsKey::NamingUnhealthyTimeoutSetSize,
        MetricsKey::NamingUnhealthyTimeoutSetItemSize,
        MetricsKey::NamingClientInstanceSetKeySize,
        MetricsKey::NamingClientInstanceSetValueSize,
        MetricsKey::NamingIndexTenantSize,
        MetricsKey::NamingIndexGroupSize,
        MetricsKey::NamingIndexServiceSize,
        //grpc
        MetricsKey::GrpcConnSize,
        MetricsKey::GrpcConnActiveTimeoutSetItemSize,
        MetricsKey::GrpcConnResponseTimeoutSetItemSize,
        //grpc request
        MetricsKey::GrpcRequestHandleRtHistogram,
        MetricsKey::GrpcRequestTotalCount,
        //http request
        MetricsKey::HttpRequestHandleRtHistogram,
        MetricsKey::HttpRequestTotalCount,
    ];
}

impl MetricsKey {
    pub fn get_key(&self) -> &'static str {
        match &self {
            MetricsKey::SysTotalMemory => "sys_total_memory",
            MetricsKey::AppRssMemory => "app_rss_memory",
            MetricsKey::AppVmsMemory => "app_vms_memory",
            MetricsKey::AppMemoryUsage => "app_memory_usage",
            MetricsKey::AppCpuUsage => "app_cpu_usage",
            MetricsKey::ConfigDataSize => "config_data_size",
            MetricsKey::ConfigListenerClientSize => "config_listener_client_size",
            MetricsKey::ConfigListenerKeySize => "config_listener_key_size",
            MetricsKey::ConfigSubscriberListenerKeySize => "config_subscriber_listener_key_size",
            MetricsKey::ConfigSubscriberListenerValueSize => {
                "config_subscriber_listener_value_size"
            }
            MetricsKey::ConfigSubscriberClientSize => "config_subscriber_client_size",
            MetricsKey::ConfigSubscriberClientValueSize => "config_subscriber_client_value_size",
            MetricsKey::ConfigIndexTenantSize => "config_index_tenant_size",
            MetricsKey::ConfigIndexConfigSize => "config_index_config_size",
            MetricsKey::NamingServiceSize => "naming_service_size",
            MetricsKey::NamingInstanceSize => "naming_instance_size",
            MetricsKey::NamingSubscriberListenerKeySize => "naming_subscriber_listener_key_size",
            MetricsKey::NamingSubscriberListenerValueSize => {
                "naming_subscriber_listener_value_size"
            }
            MetricsKey::NamingSubscriberClientSize => "naming_subscriber_client_size",
            MetricsKey::NamingSubscriberClientValueSize => "naming_subscriber_client_value_size",
            MetricsKey::NamingEmptyServiceSetSize => "naming_empty_service_set_size",
            MetricsKey::NamingEmptyServiceSetItemSize => "naming_empty_service_set_item_size",
            MetricsKey::NamingInstanceMetaSetSize => "naming_instance_meta_set_size",
            MetricsKey::NamingInstanceMetaSetItemSize => "naming_instance_meta_set_item_size",
            MetricsKey::NamingHealthyTimeoutSetSize => "naming_healthy_timeout_set_size",
            MetricsKey::NamingHealthyTimeoutSetItemSize => "naming_healthy_timeout_set_item_size",
            MetricsKey::NamingUnhealthyTimeoutSetSize => "naming_unhealthy_timeout_set_size",
            MetricsKey::NamingUnhealthyTimeoutSetItemSize => {
                "naming_unhealthy_timeout_set_item_size"
            }
            MetricsKey::NamingClientInstanceSetKeySize => "naming_client_instance_set_key_size",
            MetricsKey::NamingClientInstanceSetValueSize => "naming_client_instance_set_value_size",
            MetricsKey::NamingIndexTenantSize => "naming_index_tenant_size",
            MetricsKey::NamingIndexGroupSize => "naming_index_group_size",
            MetricsKey::NamingIndexServiceSize => "naming_index_service_size",
            MetricsKey::GrpcConnSize => "grpc_conn_size",
            MetricsKey::GrpcConnActiveTimeoutSetItemSize => {
                "grpc_conn_active_timeout_set_item_size"
            }
            MetricsKey::GrpcConnResponseTimeoutSetItemSize => {
                "grpc_conn_response_timeout_set_item_size"
            }
            MetricsKey::GrpcRequestHandleRtHistogram => "grpc_request_handle_rt_histogram",
            MetricsKey::GrpcRequestTotalCount => "grpc_request_total_count",
            MetricsKey::HttpRequestHandleRtHistogram => "http_request_handle_rt_histogram",
            MetricsKey::HttpRequestTotalCount => "http_request_total_count",
        }
    }

    /*
    pub fn get_metrics_type(&self) -> MetricsType {
        match &self {
            MetricsKey::ConfigDataSize => MetricsType::Gauge,
            MetricsKey::ConfigListenerClientSize => MetricsType::Gauge,
            MetricsKey::ConfigListenerKeySize => MetricsType::Gauge,
            MetricsKey::ConfigSubscriberListenerKeySize => MetricsType::Gauge,
            MetricsKey::ConfigSubscriberListenerValueSize => MetricsType::Gauge,
            MetricsKey::ConfigSubscriberClientSize => MetricsType::Gauge,
            MetricsKey::ConfigSubscriberClientValueSize => MetricsType::Gauge,
            MetricsKey::ConfigIndexTenantSize => MetricsType::Gauge,
            MetricsKey::ConfigIndexConfigSize => MetricsType::Gauge,
            MetricsKey::NamingServiceSize => MetricsType::Gauge,
            MetricsKey::NamingInstanceSize => MetricsType::Gauge,
            MetricsKey::NamingSubscriberListenerKeySize => MetricsType::Gauge,
            MetricsKey::NamingSubscriberListenerValueSize => MetricsType::Gauge,
            MetricsKey::NamingSubscriberClientSize => MetricsType::Gauge,
            MetricsKey::NamingSubscriberClientValueSize => MetricsType::Gauge,
            MetricsKey::NamingEmptyServiceSetSize => MetricsType::Gauge,
            MetricsKey::NamingEmptyServiceSetItemSize => MetricsType::Gauge,
            MetricsKey::NamingInstanceMetaSetSize => MetricsType::Gauge,
            MetricsKey::NamingInstanceMetaSetItemSize => MetricsType::Gauge,
            MetricsKey::NamingHealthyTimeoutSetSize => MetricsType::Gauge,
            MetricsKey::NamingHealthyTimeoutSetItemSize => MetricsType::Gauge,
            MetricsKey::NamingUnhealthyTimeoutSetSize => MetricsType::Gauge,
            MetricsKey::NamingUnhealthyTimeoutSetItemSize => MetricsType::Gauge,
            MetricsKey::NamingClientInstanceSetKeySize => MetricsType::Gauge,
            MetricsKey::NamingClientInstanceSetValueSize => MetricsType::Gauge,
            MetricsKey::NamingIndexTenantSize => MetricsType::Gauge,
            MetricsKey::NamingIndexGroupSize => MetricsType::Gauge,
            MetricsKey::NamingIndexServiceSize => MetricsType::Gauge,
            MetricsKey::GrpcConnSize => MetricsType::Gauge,
            MetricsKey::GrpcConnActiveTimeoutSetItemSize => MetricsType::Gauge,
            MetricsKey::GrpcConnResponseTimeoutSetItemSize => MetricsType::Gauge,
        }
    }
     */
}
