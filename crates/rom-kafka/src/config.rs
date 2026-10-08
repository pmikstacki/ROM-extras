//! Producer configuration invariants.
use crate::KafkaDeliveryError;
use rdkafka::ClientConfig;
use std::time::Duration;
pub(crate) fn profile(
    mut config: ClientConfig,
    deadline: Duration,
) -> Result<ClientConfig, KafkaDeliveryError> {
    if deadline < Duration::from_millis(1)
        || deadline > Duration::from_secs(300)
        || config.get("transactional.id").is_some()
    {
        return Err(KafkaDeliveryError::InvalidConfiguration);
    }
    for alias in [
        "acks",
        "retries",
        "delivery.timeout.ms",
        "linger.ms",
        "compression.type",
    ] {
        config.remove(alias);
    }
    for (key, value) in [
        ("request.required.acks", "-1"),
        ("message.send.max.retries", "0"),
        ("enable.idempotence", "false"),
        ("queue.buffering.max.messages", "1"),
        ("queue.buffering.max.kbytes", "2048"),
        ("message.max.bytes", "2097152"),
        ("queue.buffering.max.ms", "0"),
        ("compression.codec", "none"),
        ("allow.auto.create.topics", "false"),
        ("delivery.report.only.error", "false"),
    ] {
        config.set(key, value);
    }
    let milliseconds = deadline.as_millis().to_string();
    let socket_milliseconds = deadline.as_millis().max(10).to_string();
    config
        .set("message.timeout.ms", &milliseconds)
        .set("request.timeout.ms", &milliseconds)
        .set("socket.timeout.ms", &socket_milliseconds);
    let native = config
        .create_native_config()
        .map_err(|_| KafkaDeliveryError::InvalidConfiguration)?;
    for (key, value) in [
        ("acks", "-1"),
        ("retries", "0"),
        ("enable.idempotence", "false"),
        ("queue.buffering.max.messages", "1"),
        ("allow.auto.create.topics", "false"),
    ] {
        if native.get(key).ok().as_deref() != Some(value) {
            return Err(KafkaDeliveryError::InvalidConfiguration);
        }
    }
    Ok(config)
}
