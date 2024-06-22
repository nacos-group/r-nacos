#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum MetricsKey {
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

impl Default for MetricsKey {
    fn default() -> Self {
        Self::None
    }
}

impl MetricsKey {
    pub fn get_key(&self) -> &'static str {
        match &self {
            MetricsKey::None => "None",
            MetricsKey::ConfigDataSize => "ConfigDataSize",
            MetricsKey::ConfigListenerSize => "ConfigListenerSize",
            MetricsKey::ConfigSubscriberListenerKeySize => "ConfigSubscriberListenerKeySize",
            MetricsKey::ConfigSubscriberListenerValueSize => "ConfigSubscriberListenerValueSize",
            MetricsKey::ConfigSubscriberClientSize => "ConfigSubscriberClientSize",
            MetricsKey::ConfigSubscriberClientValueSize => "ConfigSubscriberClientValueSize",
            MetricsKey::ConfigIndexTenantSize => "ConfigIndexTenantSize",
            MetricsKey::ConfigIndexConfigSize => "ConfigIndexConfigSize",
            MetricsKey::NamingServiceSize => "NamingServiceSize",
            MetricsKey::NamingInstanceSize => "NamingInstanceSize",
            MetricsKey::NamingSubscriberListenerKeySize => "NamingSubscriberListenerKeySize",
            MetricsKey::NamingSubscriberListenerValueSize => "NamingSubscriberListenerValueSize",
            MetricsKey::NamingSubscriberClientSize => "NamingSubscriberClientSize",
            MetricsKey::NamingSubscriberClientValueSize => "NamingSubscriberClientValueSize",
            MetricsKey::NamingEmptyServiceSetSize => "NamingEmptyServiceSetSize",
            MetricsKey::NamingEmptyServiceSetItemSize => "NamingEmptyServiceSetItemSize",
            MetricsKey::NamingInstanceMetaSetSize => "NamingInstanceMetaSetSize",
            MetricsKey::NamingInstanceMetaSetItemSize => "NamingInstanceMetaSetItemSize",
            MetricsKey::NamingHealthyTimeoutSetSize => "NamingHealthyTimeoutSetSize",
            MetricsKey::NamingHealthyTimeoutSetItemSize => "NamingHealthyTimeoutSetItemSize",
            MetricsKey::NamingUnhealthyTimeoutSetSize => "NamingUnhealthyTimeoutSetSize",
            MetricsKey::NamingUnhealthyTimeoutSetItemSize => "NamingUnhealthyTimeoutSetItemSize",
            MetricsKey::NamingClientInstanceSetKeySize => "NamingClientInstanceSetKeySize",
            MetricsKey::NamingClientInstanceSetValueSize => "NamingClientInstanceSetValueSize",
            MetricsKey::NamingIndexTenantSize => "NamingIndexTenantSize",
            MetricsKey::NamingIndexGroupSize => "NamingIndexGroupSize",
            MetricsKey::NamingIndexServiceSize => "NamingIndexServiceSize",
        }
    }
}
