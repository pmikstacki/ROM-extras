//! Independently computed signature vector and bounded rotation scenarios.
use rom::Delivery;
use rom_delivery_core::{PayloadLimit, PreparedDelivery, WebhookSigner};

fn message() -> PreparedDelivery {
    PreparedDelivery::prepare(
        Delivery {
            id: "work-17".into(),
            attempt: 1,
            payload: false,
        },
        PayloadLimit::default(),
    )
    .unwrap()
}
#[test]
fn signature_matches_independent_node_crypto_vector() {
    // Node createHmac("sha256", Buffer.alloc(32,7)), exact signed bytes:
    // work-17.1800000000.false
    let signed = WebhookSigner::new([7; 32]).sign(&message(), 1_800_000_000);
    assert_eq!(signed.id(), "work-17");
    assert_eq!(signed.timestamp(), "1800000000");
    assert_eq!(
        signed.signature(),
        "v1,WX/AeAi6pFK/OhodbgM1JIIUf65h5KltwhuSDyu/vKg="
    );
}

#[test]
fn rotation_signs_with_current_and_previous_keys_without_changing_the_message() {
    let current = WebhookSigner::new([9; 32]).sign(&message(), 1_800_000_000);
    let previous = WebhookSigner::new([7; 32]).sign(&message(), 1_800_000_000);
    let rotated = WebhookSigner::new([9; 32])
        .with_previous_key([7; 32])
        .sign(&message(), 1_800_000_000);
    assert_eq!(
        rotated.signature(),
        format!("{} {}", current.signature(), previous.signature())
    );
    assert_eq!(rotated.id(), "work-17");
    assert_ne!(
        current.signature(),
        WebhookSigner::new([9; 32])
            .sign(&message(), 1_800_000_001)
            .signature()
    );
}
