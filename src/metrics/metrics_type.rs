
#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum MetricsType {
    None,
    ConfigDataSize,
    ConfigListenerSize,
}

impl Default for MetricsType {
    fn default() -> Self {
        Self::None
    }
}
