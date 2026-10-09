//! Core mapping construction, not provider or native authorization qualification.
mod support;
use rom::{JournalView, Key, ProjectedView, Value, json};
use rom_projection_core::{DocumentMapping, Error};
fn event(value: Option<Value>) -> JournalView {
    JournalView {
        position: 2,
        view: ProjectedView {
            key: Key {
                kind: "document".into(),
                id: "private-id".into(),
            },
            revision: 1,
            value: value.map(|v| v.as_object().unwrap().clone()),
        },
    }
}
fn mapping(fields: &[&str]) -> DocumentMapping {
    DocumentMapping::new(
        support::profile(),
        fields.iter().map(|f| f.to_string()).collect(),
        None,
    )
    .unwrap()
}
#[test]
fn only_selected_authorized_fields_are_retained_and_position_is_not_content() {
    let m = mapping(&["public", "missing"]);
    let mut e = event(Some(json!({"public":null,"private":"private-source"})));
    let d = m.document(&e, None).unwrap();
    assert_eq!(d.fields().unwrap().len(), 1);
    assert_eq!(d.fields().unwrap().get("public"), Some(&Value::Null));
    e.position = 9;
    assert_eq!(
        m.document(&e, None).unwrap().metadata().digest(),
        d.metadata().digest()
    );
    assert!(!format!("{d:?}").contains("private"));
}
#[test]
fn absent_null_false_zero_empty_and_integer_precision_remain_distinct() {
    let m = mapping(&["a"]);
    let mut digests = std::collections::BTreeSet::new();
    for v in [
        json!({}),
        json!({"a":null}),
        json!({"a":false}),
        json!({"a":0}),
        json!({"a":0.0}),
        json!({"a":""}),
        json!({"a":[]}),
        json!({"a":{}}),
        json!({"a":u64::MAX}),
        json!({"a":u64::MAX-1}),
    ] {
        assert!(
            digests.insert(
                *m.document(&event(Some(v)), None)
                    .unwrap()
                    .metadata()
                    .digest()
            )
        );
    }
}
#[test]
fn configuration_and_array_order_change_content_but_object_order_does_not() {
    let e = event(Some(json!({"a":{"z":1,"b":[1,2]}})));
    let d = mapping(&["a"]).document(&e, None).unwrap();
    assert_ne!(
        d.metadata().digest(),
        mapping(&["a", "missing"])
            .document(&e, None)
            .unwrap()
            .metadata()
            .digest()
    );
    let ordered = event(Some(json!({"a":{"b":[1,2],"z":1}})));
    assert_eq!(
        d.metadata().digest(),
        mapping(&["a"])
            .document(&ordered, None)
            .unwrap()
            .metadata()
            .digest()
    );
    let reverse = event(Some(json!({"a":{"b":[2,1],"z":1}})));
    assert_ne!(
        d.metadata().digest(),
        mapping(&["a"])
            .document(&reverse, None)
            .unwrap()
            .metadata()
            .digest()
    );
}
#[test]
fn tombstones_clear_fields_and_vectors_and_vector_profile_is_exact() {
    let p =
        rom_projection_core::ProjectionProfile::new("d", "qdrant", "m", Some("model-v1")).unwrap();
    let m = DocumentMapping::new(p, vec!["a".into()], Some(2)).unwrap();
    let e = event(Some(json!({"a":1})));
    assert!(matches!(m.document(&e, None), Err(Error::Invalid)));
    for v in [vec![1.0], vec![f32::NAN, 1.0], vec![f32::INFINITY, 1.0]] {
        assert!(matches!(m.document(&e, Some(v)), Err(Error::Invalid)));
    }
    let d = m.document(&e, Some(vec![-0.0, 1.0])).unwrap();
    assert_eq!(d.vector().unwrap()[0].to_bits(), 0);
    let t = m.document(&event(None), None).unwrap();
    assert!(t.fields().is_none());
    assert!(t.vector().is_none());
    assert!(t.metadata().is_tombstone());
    assert!(matches!(
        m.document(&event(None), Some(vec![0.0, 0.0])),
        Err(Error::Invalid)
    ));
}
#[test]
fn oversize_and_deep_selected_values_are_rejected_but_unselected_values_are_not_copied() {
    let m = mapping(&["a"]);
    let e = event(Some(json!({"a":"x".repeat(17000)})));
    assert!(matches!(m.document(&e, None), Err(Error::TooLarge)));
    let mut v = json!(0);
    for _ in 0..40 {
        v = json!([v]);
    }
    assert!(matches!(
        m.document(&event(Some(json!({"a":v}))), None),
        Err(Error::TooLarge)
    ));
    assert!(
        m.document(&event(Some(json!({"private":"x".repeat(100000)}))), None)
            .is_ok()
    );
}
#[test]
fn duplicate_fields_and_model_dimension_mismatch_are_rejected() {
    assert!(matches!(
        DocumentMapping::new(support::profile(), vec!["a".into(), "a".into()], None),
        Err(Error::Conflict)
    ));
    assert!(matches!(
        DocumentMapping::new(support::profile(), vec![], Some(2)),
        Err(Error::Invalid)
    ));
}

#[test]
fn changed_selector_is_a_different_durable_profile_not_an_in_place_mapping() {
    use rom_projection_core::{CheckpointStore, PendingHistory};
    let a = mapping(&["a"]);
    let b = mapping(&["a", "b"]);
    assert_ne!(a.profile(), b.profile());
    let dir = support::Directory::new();
    drop(CheckpointStore::create(&dir.file(), a.profile(), &support::initial()).unwrap());
    assert!(matches!(
        CheckpointStore::open(&dir.file(), b.profile()),
        Err(Error::Unsupported)
    ));
    let mut store = CheckpointStore::open(&dir.file(), a.profile()).unwrap();
    let e = event(Some(json!({"a":7,"b":8})));
    let d = a.document(&e, None).unwrap();
    let page = rom_projection_core::PageIntent::new(
        &support::initial(),
        support::cursor(3),
        vec![d.metadata().clone()],
    )
    .unwrap();
    store.prepare_page(&page).unwrap();
    drop(store);
    let store = CheckpointStore::open(&dir.file(), a.profile()).unwrap();
    let mut history =
        PendingHistory::new(store.load().unwrap().pending_page().unwrap().clone()).unwrap();
    history
        .push(
            &rom::JournalBatch {
                events: vec![e],
                cursor: support::cursor(3),
            },
            &rom_projection_core::Cancellation::new(),
            |e| Ok(b.document(e, None)?.metadata().clone()),
        )
        .unwrap();
    assert_eq!(history.finish(), Err(Error::RebuildRequired));
    assert_eq!(store.load().unwrap().pending_page(), Some(&page));
}

#[test]
fn largest_vector_and_canonical_zero_are_supported_with_finite_bounds() {
    let p = rom_projection_core::ProjectionProfile::new("d", "qdrant", "m", Some("model")).unwrap();
    let m = DocumentMapping::new(p, vec![], Some(4096)).unwrap();
    let e = event(Some(json!({})));
    let a = m.document(&e, Some(vec![0.0; 4096])).unwrap();
    let b = m.document(&e, Some(vec![-0.0; 4096])).unwrap();
    assert_eq!(a.metadata().digest(), b.metadata().digest());
    assert_eq!(a.vector().unwrap().len(), 4096);
    assert!(matches!(
        m.document(&e, Some(vec![0.0; 4097])),
        Err(Error::Invalid)
    ));
}
