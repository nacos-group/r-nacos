#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum MetricsType {
    None,
    //config
    ConfigDataSize,
    ConfigListenerSize,
    ConfigSubscriberListenerKeySize,
    ConfigSubscriberListenerValueSize,
    ConfigSubscriberListenerClientSize,
    ConfigSubscriberListenerClientKeySize,
    ConfigIndexTenantSize,
    ConfigIndexConfigSize,
    //naming
    NamingServiceSize,
    NamingInstanceSize,
    NamingTimeInfoSize,
    NamingSubscriberListenerKeySize,
    NamingSubscriberListenerValueSize,
}

impl Default for MetricsType {
    fn default() -> Self {
        Self::None
    }
}
