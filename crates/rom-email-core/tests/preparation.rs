//! Public email preparation, policy, codec and diagnostic boundaries.
use rom::{Delivery, Input};
use rom_email_core::{EmailError, EmailNotification, EmailProfile};
fn payload() -> EmailNotification {
    EmailNotification::new(
        "Exact+Tag@example.invalid",
        "Zażółć",
        "Treść\n\n.\n",
        1800000000,
    )
    .unwrap()
}
fn profile(limit: usize) -> EmailProfile {
    EmailProfile::new(
        "sender@example.invalid",
        &["Exact+Tag@example.invalid"],
        "host.example.invalid",
        limit,
    )
    .unwrap()
}
fn delivery(attempt: u32) -> Delivery<EmailNotification> {
    Delivery {
        id: "work-17".into(),
        attempt,
        payload: payload(),
    }
}
#[test]
fn retry_preserves_exact_rfc_bytes_and_envelope() {
    let p = profile(1048576);
    let a = p.prepare(delivery(1)).unwrap();
    let b = p.prepare(delivery(2)).unwrap();
    assert_eq!(a.body(), b.body());
    assert_eq!(a.message_id(), b.message_id());
    assert_eq!(a.from(), "sender@example.invalid");
    assert_eq!(a.to(), "Exact+Tag@example.invalid");
    let wire = String::from_utf8(a.body().to_vec()).unwrap();
    assert!(wire.contains("Content-Type: text/plain; charset=utf-8"));
    assert!(wire.contains("MIME-Version: 1.0\r\n"));
    assert!(wire.contains("Date:"));
    assert!(!wire.contains("work-17"));
    assert!(a.message_id().ends_with("@host.example.invalid>"));
}
#[test]
fn restored_payload_is_validated_and_unknown_fields_are_rejected() {
    let a = payload();
    let v = a.encode();
    assert_eq!(EmailNotification::decode(v.clone()).unwrap().encode(), v);
    for key in ["to", "subject", "text", "created_unix_seconds"] {
        let mut bad = v.clone();
        bad.as_object_mut().unwrap().remove(key);
        assert!(EmailNotification::decode(bad).is_err());
    }
    let mut bad = v.clone();
    bad["secret"] = true.into();
    assert!(EmailNotification::decode(bad).is_err());
    let mut bad = v.clone();
    bad["subject"] = "bad\r\nBcc: victim@example.invalid".into();
    assert!(EmailNotification::decode(bad).is_err());
    let mut bad = v;
    bad["created_unix_seconds"] = (-1).into();
    assert!(EmailNotification::decode(bad).is_err());
}
#[test]
fn host_recipient_policy_matches_exact_addresses() {
    let mut d = delivery(1);
    d.payload = EmailNotification::new("exact+tag@example.invalid", "ok", "", 0).unwrap();
    assert!(matches!(
        profile(1048576).prepare(d),
        Err(EmailError::RecipientDenied)
    ));
    assert!(
        EmailProfile::new("sender@example.invalid", &[], "host.example.invalid", 1024).is_err()
    );
    assert!(
        EmailProfile::new(
            "sender@example.invalid",
            &["a@example.invalid", "a@example.invalid"],
            "host.example.invalid",
            1024
        )
        .is_err()
    );
}
#[test]
fn headers_and_mailboxes_reject_injection_and_unsupported_forms() {
    for to in [
        "",
        "a@example.invalid,b@example.invalid",
        "Name <a@example.invalid>",
        "a@example.invalid\r\nBcc: b@example.invalid",
        "ü@example.invalid",
    ] {
        assert!(EmailNotification::new(to, "ok", "", 0).is_err(), "{to:?}");
    }
    for subject in ["", "a\nBcc: bad", "a\0b", "a\tb"] {
        assert!(EmailNotification::new("a@example.invalid", subject, "", 0).is_err());
    }
    for text in ["a\0b", "a\u{1b}b", "a\rb"] {
        assert!(EmailNotification::new("a@example.invalid", "ok", text, 0).is_err());
    }
    for domain in [
        "localhost",
        "a..invalid",
        "-a.invalid",
        "a.invalid\r\nX: b",
        "a.invalid:25",
    ] {
        assert!(
            EmailProfile::new("a@example.invalid", &["a@example.invalid"], domain, 1024).is_err()
        );
    }
}
#[test]
fn input_and_encoded_wire_limits_are_enforced() {
    assert!(EmailNotification::new("a@example.invalid", &"a".repeat(512), "", 4102444800).is_ok());
    assert!(EmailNotification::new("a@example.invalid", &"a".repeat(513), "", 0).is_err());
    assert!(EmailNotification::new("a@example.invalid", "ok", &"a".repeat(262145), 0).is_err());
    assert!(EmailNotification::new("a@example.invalid", "ok", "", 4102444801).is_err());
    let n = profile(1048576).prepare(delivery(1)).unwrap().body().len();
    assert_eq!(profile(n).prepare(delivery(1)).unwrap().body().len(), n);
    assert!(matches!(
        profile(n - 1).prepare(delivery(1)),
        Err(EmailError::TooLarge)
    ));
    assert!(
        EmailProfile::new(
            "a@example.invalid",
            &["a@example.invalid"],
            "host.example.invalid",
            0
        )
        .is_err()
    );
    assert!(
        EmailProfile::new(
            "a@example.invalid",
            &["a@example.invalid"],
            "host.example.invalid",
            1048577
        )
        .is_err()
    );
}
#[test]
fn diagnostics_do_not_disclose_personal_fields_or_delivery_identity() {
    let p = profile(1048576);
    let a = payload();
    let m = p.prepare(delivery(1)).unwrap();
    for debug in [format!("{p:?}"), format!("{a:?}"), format!("{m:?}")] {
        for secret in [
            "sender@example.invalid",
            "Exact+Tag@example.invalid",
            "Zażółć",
            "Treść",
            "work-17",
        ] {
            assert!(!debug.contains(secret));
        }
    }
    for id in ["", "injected\r\n", "a.b"] {
        let mut d = delivery(1);
        d.id = id.into();
        assert!(matches!(p.prepare(d), Err(EmailError::InvalidIdentity)));
    }
}
