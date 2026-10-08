//! Host-owned configuration for the dedicated TLS/SASL fixture.
use rdkafka::ClientConfig;
pub fn config() -> ClientConfig {
    let brokers = std::env::var("ROM_EXTRAS_KAFKA_TLS_BROKERS").expect("required TLS brokers");
    assert_eq!(brokers, "localhost:55450", "dedicated TLS fixture required");
    let mut cfg = ClientConfig::new();
    cfg.set("bootstrap.servers", brokers)
        .set("broker.address.family", "v4")
        .set("security.protocol", "sasl_ssl")
        .set(
            "ssl.ca.location",
            std::env::var("ROM_EXTRAS_KAFKA_TLS_CA").expect("required CA"),
        )
        .set("enable.ssl.certificate.verification", "true")
        .set("ssl.endpoint.identification.algorithm", "https")
        .set("sasl.mechanism", "PLAIN")
        .set(
            "sasl.username",
            std::env::var("ROM_EXTRAS_KAFKA_TLS_USERNAME").expect("required username"),
        )
        .set(
            "sasl.password",
            std::env::var("ROM_EXTRAS_KAFKA_TLS_PASSWORD").expect("required password"),
        );
    cfg
}
