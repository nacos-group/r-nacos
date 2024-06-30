use std::borrow::Cow;
//use crate::metrics::model::MetricsType;
use lazy_static::lazy_static;

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub struct Label(pub Cow<'static, str>, pub Cow<'static, str>);

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum MetricsKey {
    //app
    ProcessStartTimeSeconds,
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
    GrpcRequestHandleRtSummary,
    GrpcRequestTotalCount,
    //http api request
    HttpRequestHandleRtHistogram,
    HttpRequestHandleRtSummary,
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
            MetricsKey::ProcessStartTimeSeconds => "process_start_time_seconds",
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
            MetricsKey::GrpcRequestHandleRtSummary => "grpc_request_handle_rt_summary",
            MetricsKey::GrpcRequestTotalCount => "grpc_request_total_count",
            MetricsKey::HttpRequestHandleRtHistogram => "http_request_handle_rt_histogram",
            MetricsKey::HttpRequestHandleRtSummary => "http_request_handle_rt_summary",
            MetricsKey::HttpRequestTotalCount => "http_request_total_count",
        }
    }

    pub fn get_labels(&self) -> Option<Vec<&Label>> {
        //todo 后续的指标key可以支持labels
        None
    }

    pub fn get_key_with_label(&self) -> Cow<'static, str> {
        let key = self.get_key();
        if let Some(_labels) = self.get_labels() {
            //todo 把key与label拼接到一起展示
            //key{label_key=label_value,label_key2=label_value2}
            Cow::Owned(key.to_string())
        } else {
            Cow::Borrowed(key)
        }
    }

    pub fn get_describe(&self) -> &'static str {
        match &self {
            MetricsKey::ProcessStartTimeSeconds => "Process start time seconds",
            MetricsKey::SysTotalMemory => "Sys total memory,unit is M",
            MetricsKey::AppRssMemory => "App rss memory,unit is M",
            MetricsKey::AppVmsMemory => "App vms memory,unit is M",
            MetricsKey::AppMemoryUsage => "App memory usage",
            MetricsKey::AppCpuUsage => "App cpu usage",
            MetricsKey::ConfigDataSize => "Config data size",
            MetricsKey::ConfigListenerClientSize => "Config listener client size",
            MetricsKey::ConfigListenerKeySize => "Config listener key size",
            MetricsKey::ConfigSubscriberListenerKeySize => "Config subscriber listener key size",
            MetricsKey::ConfigSubscriberListenerValueSize => {
                "Config subscriber listener value size"
            }
            MetricsKey::ConfigSubscriberClientSize => "Config subscriber client size",
            MetricsKey::ConfigSubscriberClientValueSize => "Config subscriber client value size",
            MetricsKey::ConfigIndexTenantSize => "Config index tenant size",
            MetricsKey::ConfigIndexConfigSize => "Config index config size",
            MetricsKey::NamingServiceSize => "Naming service size",
            MetricsKey::NamingInstanceSize => "Naming instance size",
            MetricsKey::NamingSubscriberListenerKeySize => "Naming subscriber listener key size",
            MetricsKey::NamingSubscriberListenerValueSize => {
                "Naming subscriber listener value size"
            }
            MetricsKey::NamingSubscriberClientSize => "Naming subscriber client size",
            MetricsKey::NamingSubscriberClientValueSize => "Naming subscriber client value size",
            MetricsKey::NamingEmptyServiceSetSize => "Naming empty service set size",
            MetricsKey::NamingEmptyServiceSetItemSize => "Naming empty service set item size",
            MetricsKey::NamingInstanceMetaSetSize => "Naming instance meta set size",
            MetricsKey::NamingInstanceMetaSetItemSize => "Naming instance meta set item size",
            MetricsKey::NamingHealthyTimeoutSetSize => "Naming healthy timeout set size",
            MetricsKey::NamingHealthyTimeoutSetItemSize => "Naming healthy timeout set item size",
            MetricsKey::NamingUnhealthyTimeoutSetSize => "Naming unhealthy timeout set size",
            MetricsKey::NamingUnhealthyTimeoutSetItemSize => {
                "Naming unhealthy timeout set item size"
            }
            MetricsKey::NamingClientInstanceSetKeySize => "Naming client instance set key size",
            MetricsKey::NamingClientInstanceSetValueSize => "Naming client instance set value size",
            MetricsKey::NamingIndexTenantSize => "Naming index tenant size",
            MetricsKey::NamingIndexGroupSize => "Naming index group size",
            MetricsKey::NamingIndexServiceSize => "Naming index service size",
            MetricsKey::GrpcConnSize => "Grpc conn size",
            MetricsKey::GrpcConnActiveTimeoutSetItemSize => {
                "Grpc conn active timeout set item size"
            }
            MetricsKey::GrpcConnResponseTimeoutSetItemSize => {
                "Grpc conn response timeout set item size"
            }
            MetricsKey::GrpcRequestHandleRtHistogram => {
                "Grpc request handle rt histogram,unit is ms"
            }
            MetricsKey::GrpcRequestHandleRtSummary => "Grpc request handle rt summary, unit is ms",
            MetricsKey::GrpcRequestTotalCount => "Grpc request total count",
            MetricsKey::HttpRequestHandleRtHistogram => {
                "Http request handle rt histogram,unit is ms"
            }
            MetricsKey::HttpRequestHandleRtSummary => "Http request handle rt summary,unit is ms",
            MetricsKey::HttpRequestTotalCount => "Http request total count",
            //default describe
            //_ => "Some help info",
        }
    }
}
