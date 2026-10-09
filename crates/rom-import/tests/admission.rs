//! Public admission and immutable request behavior.
use rom::{Key, Operation, RevisionCondition};
use rom_import::{ActionBinding, ActionPlan, Error, Limits, SourceGrant};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
fn plan(bytes: &[u8], limits: Limits) -> ActionPlan {
    ActionPlan::trusted(
        ActionBinding {
            target: Key {
                kind: "items".into(),
                id: "exact.id[0]".into(),
            },
            action: "ingest".into(),
            expected: Some(7),
            idempotency: "original".into(),
            retry_epoch: 9,
        },
        SourceGrant {
            source: "deployment".into(),
            generation: 3,
            output_origins: BTreeMap::from([("computed".into(), "host-attested".into())]),
            condition: RevisionCondition {
                key: Key {
                    kind: "controls".into(),
                    id: "source".into(),
                },
                revision: 4,
            },
            valid_until: u64::MAX,
            expected_sha256: Sha256::digest(bytes).into(),
        },
        limits,
    )
    .unwrap()
}
#[test]
fn changed_representation_cannot_become_an_action() {
    let p = plan(br#"{"value":1}"#, Limits::default());
    assert_eq!(
        p.prepare(br#"{"value":2}"#).unwrap_err(),
        Error::DigestMismatch
    );
}
#[test]
fn retained_request_preserves_identity_and_complete_input() {
    let bytes = br#"{"n":18446744073709551615,"null":null,"false":false,"zero":0,"empty":""}"#;
    let prepared = plan(bytes, Limits::default()).prepare(bytes).unwrap();
    let (a, permit) = prepared.request();
    let (b, same) = prepared.request();
    assert_eq!(a, b);
    assert!(permit == same);
    assert_eq!(a.id, "exact.id[0]");
    assert_eq!(a.expected, Some(7));
    assert_eq!(a.retry_epoch, 9);
    assert_eq!(a.idempotency, "original");
    let Operation::Action { name, input } = a.operation else {
        panic!("action required")
    };
    assert_eq!(name, "ingest");
    assert_eq!(input["n"].as_u64(), Some(u64::MAX));
    assert!(input.get("missing").is_none());
    assert!(input["null"].is_null());
    assert_eq!(input["false"].as_bool(), Some(false));
    assert_eq!(input["zero"].as_u64(), Some(0));
    assert_eq!(input["empty"].as_str(), Some(""));
}
#[test]
fn malformed_duplicate_and_trailing_documents_are_refused() {
    for bytes in [
        b"{SECRET".as_slice(),
        br#"{"a":{"k":1,"k":2}}"#,
        b"{} false",
        &[255],
    ] {
        assert_eq!(
            plan(bytes, Limits::default()).prepare(bytes).unwrap_err(),
            Error::InvalidJson
        );
    }
}
#[test]
fn limits_reject_before_request_creation() {
    assert!(Limits::new(0, 1, 1, 1).is_err());
    assert!(Limits::new(65537, 1, 1, 1).is_err());
    for (bytes, limits) in [
        (b"true".as_slice(), Limits::new(3, 32, 4096, 16384).unwrap()),
        (b"[[0]]", Limits::new(100, 2, 4096, 16384).unwrap()),
        (b"[0,1]", Limits::new(100, 32, 2, 16384).unwrap()),
        (br#""long""#, Limits::new(100, 32, 4096, 3).unwrap()),
        (br#"{"long":0}"#, Limits::new(100, 32, 4096, 3).unwrap()),
    ] {
        assert_eq!(
            plan(bytes, limits).prepare(bytes).unwrap_err(),
            Error::TooLarge
        );
    }
    assert!(
        plan(b"[0]", Limits::new(3, 2, 2, 1).unwrap())
            .prepare(b"[0]")
            .is_ok()
    );
}
#[test]
fn diagnostics_hide_payload_and_grants() {
    let bytes = br#""SECRET""#;
    let p = plan(bytes, Limits::default());
    let prepared = p.prepare(bytes).unwrap();
    for text in [
        format!("{p:?}"),
        format!("{prepared:?}"),
        Error::InvalidJson.to_string(),
    ] {
        for sensitive in [
            "SECRET",
            "exact.id",
            "deployment",
            "host-attested",
            "original",
        ] {
            assert!(!text.contains(sensitive));
        }
    }
}
