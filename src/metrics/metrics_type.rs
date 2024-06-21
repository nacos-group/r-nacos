#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum MetricsType {
    None,
    //config
    ConfigDataSize,
    ConfigListenerSize,
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
}

impl Default for MetricsType {
    fn default() -> Self {
        Self::None
    }
}

impl MetricsType {
    pub fn get_key(&self) -> &'static str {
        match &self {
            MetricsType::None => "None",
            MetricsType::ConfigDataSize => "ConfigDataSize",
            MetricsType::ConfigListenerSize => "ConfigListenerSize",
            MetricsType::ConfigSubscriberListenerKeySize => "ConfigSubscriberListenerKeySize",
            MetricsType::ConfigSubscriberListenerValueSize => "ConfigSubscriberListenerValueSize",
            MetricsType::ConfigSubscriberClientSize => "ConfigSubscriberClientSize",
            MetricsType::ConfigSubscriberClientValueSize => "ConfigSubscriberClientValueSize",
            MetricsType::ConfigIndexTenantSize => "ConfigIndexTenantSize",
            MetricsType::ConfigIndexConfigSize => "ConfigIndexConfigSize",
            MetricsType::NamingServiceSize => "NamingServiceSize",
            MetricsType::NamingInstanceSize => "NamingInstanceSize",
            MetricsType::NamingSubscriberListenerKeySize => "NamingSubscriberListenerKeySize",
            MetricsType::NamingSubscriberListenerValueSize => "NamingSubscriberListenerValueSize",
            MetricsType::NamingSubscriberClientSize => "NamingSubscriberClientSize",
            MetricsType::NamingSubscriberClientValueSize => "NamingSubscriberClientValueSize",
            MetricsType::NamingEmptyServiceSetSize => "NamingEmptyServiceSetSize",
            MetricsType::NamingEmptyServiceSetItemSize => "NamingEmptyServiceSetItemSize",
            MetricsType::NamingInstanceMetaSetSize => "NamingInstanceMetaSetSize",
            MetricsType::NamingInstanceMetaSetItemSize => "NamingInstanceMetaSetItemSize",
            MetricsType::NamingHealthyTimeoutSetSize => "NamingHealthyTimeoutSetSize",
            MetricsType::NamingHealthyTimeoutSetItemSize => "NamingHealthyTimeoutSetItemSize",
            MetricsType::NamingUnhealthyTimeoutSetSize => "NamingUnhealthyTimeoutSetSize",
            MetricsType::NamingUnhealthyTimeoutSetItemSize => "NamingUnhealthyTimeoutSetItemSize",
            MetricsType::NamingClientInstanceSetKeySize => "NamingClientInstanceSetKeySize",
            MetricsType::NamingClientInstanceSetValueSize => "NamingClientInstanceSetValueSize",
            MetricsType::NamingIndexTenantSize => "NamingIndexTenantSize",
            MetricsType::NamingIndexGroupSize => "NamingIndexGroupSize",
            MetricsType::NamingIndexServiceSize => "NamingIndexServiceSize",
        }
    }
}
