/// Kafka construction failure without credential-bearing client diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KafkaDeliveryError {
    /// Topic, deadline, transaction configuration, or native property is invalid.
    InvalidConfiguration,
    /// Native client construction failed. Configuration values are not disclosed.
    Unavailable,
}
impl std::fmt::Display for KafkaDeliveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidConfiguration => "invalid Kafka delivery configuration",
            Self::Unavailable => "Kafka producer unavailable",
        })
    }
}
impl std::error::Error for KafkaDeliveryError {}
