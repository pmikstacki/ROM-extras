//! Payload distinctions, byte limits, and stable delivery identity scenarios.
use rom::{Delivery, Input, Value};
use rom_delivery_core::{DeliveryError, PayloadLimit, PreparedDelivery};

#[derive(Clone)]
struct Payload(Value);
impl Input for Payload {
    fn encode(&self) -> Value {
        self.0.clone()
    }
    fn decode(value: Value) -> rom::Result<Self> {
        Ok(Self(value))
    }
}
fn delivery(value: Value) -> Delivery<Payload> {
    Delivery {
        id: "work-17".into(),
        attempt: 1,
        payload: Payload(value),
    }
}

#[test]
fn scalar_payloads_keep_null_false_zero_and_empty_distinct() {
    for (value, expected) in [
        (Value::Null, "null"),
        (Value::Bool(false), "false"),
        (Value::from(0), "0"),
        (Value::from(""), "\"\""),
    ] {
        let prepared = PreparedDelivery::prepare(delivery(value), PayloadLimit::default()).unwrap();
        assert_eq!(prepared.body(), expected.as_bytes());
        assert_eq!(prepared.id(), "work-17");
        assert_eq!(prepared.attempt(), 1);
    }
}

#[test]
fn encoded_byte_limit_includes_json_quotes_escaping_and_utf8() {
    let exact = PayloadLimit::new(4).unwrap();
    assert_eq!(
        PreparedDelivery::prepare(delivery(Value::from("é")), exact)
            .unwrap()
            .body(),
        "\"é\"".as_bytes()
    );
    assert!(matches!(
        PreparedDelivery::prepare(delivery(Value::from("é")), PayloadLimit::new(3).unwrap()),
        Err(DeliveryError::TooLarge)
    ));
    assert!(matches!(
        PreparedDelivery::prepare(delivery(Value::from("\n")), PayloadLimit::new(3).unwrap()),
        Err(DeliveryError::TooLarge)
    ));
    assert!(PayloadLimit::new(0).is_err());
    assert!(PayloadLimit::new(1_048_577).is_err());
}

#[derive(Clone)]
struct UnreachablePayload;
impl Input for UnreachablePayload {
    fn encode(&self) -> Value {
        panic!("invalid identity must reject before encoding")
    }
    fn decode(_: Value) -> rom::Result<Self> {
        Ok(Self)
    }
}
#[test]
fn invalid_identity_rejects_before_payload_encoding() {
    for id in [
        "",
        "injected\r\nheader",
        "two words",
        "ambiguous.id",
        "non-ascii-é",
    ] {
        assert!(matches!(
            PreparedDelivery::prepare(
                Delivery {
                    id: id.into(),
                    attempt: 1,
                    payload: UnreachablePayload
                },
                PayloadLimit::default()
            ),
            Err(DeliveryError::InvalidIdentity)
        ));
    }
    assert!(matches!(
        PreparedDelivery::prepare(
            Delivery {
                id: "a".repeat(2049),
                attempt: 1,
                payload: UnreachablePayload
            },
            PayloadLimit::default()
        ),
        Err(DeliveryError::InvalidIdentity)
    ));
}

#[test]
fn retry_keeps_identity_and_payload_but_exposes_the_runtime_attempt() {
    let first =
        PreparedDelivery::prepare(delivery(Value::Bool(false)), PayloadLimit::default()).unwrap();
    let mut retry = delivery(Value::Bool(false));
    retry.attempt = 2;
    let second = PreparedDelivery::prepare(retry, PayloadLimit::default()).unwrap();
    assert_eq!(first.id(), second.id());
    assert_eq!(first.body(), second.body());
    assert_eq!(second.attempt(), 2);
    assert_eq!(second.into_body(), b"false");
}

#[test]
fn prepared_debug_output_does_not_disclose_payload_values() {
    let prepared = PreparedDelivery::prepare(
        delivery(Value::from("protected-payload-value")),
        PayloadLimit::default(),
    )
    .unwrap();
    assert!(!format!("{prepared:?}").contains("protected-payload-value"));
}
