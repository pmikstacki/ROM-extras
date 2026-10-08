/// Binding errors omit connection credentials and broker payloads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RabbitMqError {
    /// The queue name or operation deadline is outside the supported profile.
    InvalidConfiguration,
    /// Channel creation, queue existence probing, or confirm selection failed.
    Unavailable,
}
impl std::fmt::Display for RabbitMqError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidConfiguration => "invalid RabbitMQ delivery configuration",
            Self::Unavailable => "RabbitMQ delivery binding unavailable",
        })
    }
}
impl std::error::Error for RabbitMqError {}
