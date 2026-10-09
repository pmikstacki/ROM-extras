//! Catch loss of original-key fencing, u64 precision, and retained tombstone values.
use crate::PreparedWrite;
use rom::{JournalView, Key, ProjectedView, json};
use rom_projection_core::{ApprovedDocument, DocumentMapping, ProjectionProfile};
use serde_json::Value;

fn document(revision: u64, tombstone: bool) -> ApprovedDocument {
    let mapping = DocumentMapping::new(
        ProjectionProfile::new("deployment", "qdrant", "mapping", Some("model")).unwrap(),
        vec!["value".into()],
        Some(3),
    )
    .unwrap();
    mapping
        .document(
            &JournalView {
                position: 1,
                view: ProjectedView {
                    key: Key {
                        kind: "documents".into(),
                        id: "original".into(),
                    },
                    revision,
                    value: (!tombstone)
                        .then(|| json!({"value":u64::MAX}).as_object().unwrap().clone()),
                },
            },
            (!tombstone).then(|| vec![1.0, 0.0, 0.0]),
        )
        .unwrap()
}
fn body(revision: u64, tombstone: bool) -> Value {
    serde_json::from_slice(
        PreparedWrite::new(&document(revision, tombstone))
            .unwrap()
            .as_bytes(),
    )
    .unwrap()
}
#[test]
fn conditional_revision_update_also_binds_original_key() {
    let b = body(u64::MAX, false);
    assert_eq!(
        b["update_filter"]["must"][0],
        json!({"key":"rom_kind","match":{"value":"documents"}})
    );
    assert_eq!(
        b["update_filter"]["must"][1],
        json!({"key":"rom_id","match":{"value":"original"}})
    );
    assert_eq!(
        b["update_filter"]["must"][2]["should"],
        json!([
            {"key":"rom_revision_hi","range":{"lt":4294967295_u64}},
            {"must":[{"key":"rom_revision_hi","match":{"value":4294967295_u64}},
                     {"key":"rom_revision_lo","range":{"lt":4294967295_u64}}]}
        ])
    );
    assert_eq!(b["points"][0]["payload"]["rom_revision_hi"], 4294967295_u64);
    assert_eq!(b["points"][0]["payload"]["rom_revision_lo"], 4294967295_u64);
    assert_eq!(b["update_mode"], "upsert");
}
#[test]
fn tombstone_retains_identity_and_revision_but_removes_values_and_vectors() {
    let b = body(1_u64 << 32, true);
    let point = &b["points"][0];
    assert_eq!(point["vector"], json!({}));
    assert_eq!(point["payload"]["rom_live"], false);
    assert_eq!(point["payload"]["rom_revision_hi"], 1);
    assert_eq!(point["payload"]["rom_revision_lo"], 0);
    assert!(point["payload"].get("rom_values").is_none());
    assert_eq!(point["payload"]["rom_id"], "original");
}
#[test]
fn approved_json_is_stored_losslessly_without_native_numeric_coercion() {
    let b = body(9, false);
    let retained: Value =
        serde_json::from_str(b["points"][0]["payload"]["rom_values"].as_str().unwrap()).unwrap();
    assert_eq!(retained, json!({"value":u64::MAX}));
    assert_eq!(b["points"][0]["vector"], json!({"embedding":[1.0,0.0,0.0]}));
}
