use crate::config::profile;
use rdkafka::ClientConfig;
use std::time::Duration;
#[test]
fn native_aliases_cannot_weaken_acknowledgement_or_retry_bounds() {
    let mut host = ClientConfig::new();
    host.set("acks", "0")
        .set("request.required.acks", "1")
        .set("retries", "99")
        .set("enable.idempotence", "true")
        .set("queue.buffering.max.messages", "0")
        .set("delivery.timeout.ms", "0");
    let cfg = profile(host, Duration::from_secs(2)).unwrap();
    let native = cfg.create_native_config().unwrap();
    assert_eq!(native.get("acks").unwrap(), "-1");
    assert_eq!(native.get("retries").unwrap(), "0");
    assert_eq!(native.get("enable.idempotence").unwrap(), "false");
    assert_eq!(native.get("queue.buffering.max.messages").unwrap(), "1");
    assert_eq!(native.get("delivery.timeout.ms").unwrap(), "2000");
    assert_eq!(native.get("allow.auto.create.topics").unwrap(), "false");
}
#[test]
fn transactional_configuration_is_refused_without_reinterpreting_it() {
    let mut config = ClientConfig::new();
    config.set("transactional.id", "host-transaction");
    assert!(profile(config, Duration::from_secs(2)).is_err());
}
#[test]
fn minimum_deadline_remains_finite_and_native_socket_timeout_is_valid() {
    assert!(profile(ClientConfig::new(), Duration::from_micros(999)).is_err());
    let cfg = profile(ClientConfig::new(), Duration::from_millis(1)).unwrap();
    let native = cfg.create_native_config().unwrap();
    assert_eq!(native.get("message.timeout.ms").unwrap(), "1");
    assert_eq!(native.get("socket.timeout.ms").unwrap(), "10");
    assert_eq!(native.get("delivery.report.only.error").unwrap(), "false");
}
