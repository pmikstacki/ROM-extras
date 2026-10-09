//! Actual persistent verified-mTLS OpenSearch write qualification.
#![cfg(feature = "service-fixture")]
#[path = "support/approved_document.rs"]
mod approved_document;
#[path = "support/fixture_process.rs"]
mod fixture_process;
#[path = "support/native_fixture.rs"]
mod native_fixture;
use approved_document::document;
use native_fixture::fixture;
use rom_projection_core::{ProjectionTarget, TargetFailure};
#[tokio::test(flavor = "current_thread")]
async fn actual_external_replay_conflict_stale_and_tombstone() {
    let (mut target, mapping) = fixture();
    native_fixture::create_generation(&mut target).await;
    let a = document(&mapping, 1, Some("first"));
    let request = target.prepare(&[&a]).unwrap();
    assert_eq!(target.apply(request).await.unwrap().len(), 1);
    let request = target.prepare(&[&a]).unwrap();
    assert_eq!(target.apply(request).await.unwrap().len(), 1);
    let conflicting = document(&mapping, 1, Some("different"));
    let request = target.prepare(&[&conflicting]).unwrap();
    assert!(matches!(
        target.apply(request).await,
        Err(TargetFailure::Rejected)
    ));
    let tomb = document(&mapping, 2, None);
    let request = target.prepare(&[&tomb]).unwrap();
    assert_eq!(target.apply(request).await.unwrap().len(), 1);
    let request = target.prepare(&[&a]).unwrap();
    assert!(matches!(
        target.apply(request).await,
        Err(TargetFailure::Rejected)
    ));
}
#[tokio::test(flavor = "current_thread")]
async fn signed_revision_overflow_rejected_before_dispatch() {
    let (target, mapping) = fixture();
    let a = document(&mapping, i64::MAX as u64 + 1, Some("overflow"));
    assert!(matches!(
        target.prepare(&[&a]),
        Err(rom_projection_core::Error::Unsupported)
    ));
}

#[tokio::test(flavor = "current_thread")]
async fn changed_approved_document_profile_fails_before_write() {
    let (mut target, mapping) = fixture();
    native_fixture::create_generation(&mut target).await;
    // A changed core mapping profile cannot reuse this configured adapter.
    let a = document(&mapping, 1, Some("first"));
    let request = target.prepare(&[&a]).unwrap();
    assert_eq!(target.apply(request).await.unwrap().len(), 1);
    let different = rom_projection_core::DocumentMapping::new(
        rom_projection_core::ProjectionProfile::new("native-test", "opensearch", "different", None)
            .unwrap(),
        vec!["title".into()],
        None,
    )
    .unwrap();
    let a = document(&different, 2, Some("changed"));
    assert!(matches!(
        target.prepare(&[&a]),
        Err(rom_projection_core::Error::Conflict)
    ));
}
#[tokio::test(flavor = "current_thread")]
async fn signed_maximum_and_exact_unindexed_json_round_trip() {
    let (mut target, mapping) = fixture();
    native_fixture::create_generation(&mut target).await;
    let a = document(&mapping, i64::MAX as u64, Some("largest admitted revision"));
    let request = target.prepare(&[&a]).unwrap();
    assert_eq!(target.apply(request).await.unwrap().len(), 1);
    // Exact source equality in apply includes the unindexed u64::MAX, not only metadata.
}
#[test]
fn page_and_selected_text_bounds_reject_without_native_intent_or_network() {
    let (target, mapping) = fixture();
    let a = document(&mapping, 1, Some("one"));
    assert!(matches!(
        target.prepare(&vec![&a; 65]),
        Err(rom_projection_core::Error::TooLarge)
    ));
    assert!(matches!(
        target.prepare(&[&a, &a]),
        Err(rom_projection_core::Error::Conflict)
    ));
    let bad = mapping
        .document(
            &rom::JournalView {
                position: 1,
                view: rom::ProjectedView {
                    key: rom::Key {
                        kind: "native_docs".into(),
                        id: "number".into(),
                    },
                    revision: 1,
                    value: Some(
                        serde_json::json!({"title":false})
                            .as_object()
                            .unwrap()
                            .clone(),
                    ),
                },
            },
            None,
        )
        .unwrap();
    assert!(matches!(
        target.prepare(&[&bad]),
        Err(rom_projection_core::Error::Unsupported)
    ));
}

#[tokio::test(flavor = "current_thread")]
async fn actual_response_loss_replay_inspects_native_committed_state() {
    let proxy = loss_proxy::LossProxy::start();
    let (mut direct, mapping) = fixture();
    native_fixture::create_generation(&mut direct).await;
    let mut lossy = loss_proxy::lossy_target(&proxy, &mapping, direct.physical_target());
    let a = document(&mapping, 1, Some("accepted before acknowledgement loss"));
    let request = lossy.prepare(&[&a]).unwrap();
    assert!(matches!(
        lossy.apply(request).await,
        Err(TargetFailure::Unknown)
    ));
    let request = direct.prepare(&[&a]).unwrap();
    assert_eq!(direct.inspect_prepared(&request).await.unwrap().len(), 1);
    assert_eq!(direct.apply(request).await.unwrap().len(), 1);
}

#[path = "support/loss_proxy.rs"]
mod loss_proxy;

#[tokio::test(flavor = "current_thread")]
async fn late_native_request_after_timeout_cannot_regress_live_or_tombstone() {
    for tombstone in [false, true] {
        let proxy = loss_proxy::LossProxy::delayed();
        let (mut direct, mapping) = fixture();
        native_fixture::create_generation(&mut direct).await;
        let mut delayed = loss_proxy::lossy_target(&proxy, &mapping, direct.physical_target());
        let old = document(&mapping, 1, Some("delayed old representation"));
        let request = delayed.prepare(&[&old]).unwrap();
        let pending = tokio::spawn(async move { delayed.apply(request).await });
        proxy.marker("held").await;
        assert!(matches!(
            pending.await.unwrap(),
            Err(TargetFailure::Unknown)
        ));
        let newer = document(
            &mapping,
            2,
            if tombstone {
                None
            } else {
                Some("newer representation")
            },
        );
        let request = direct.prepare(&[&newer]).unwrap();
        assert_eq!(direct.apply(request).await.unwrap().len(), 1);
        let done: serde_json::Value = serde_json::from_str(&proxy.marker("done").await).unwrap();
        assert_eq!(done["http"], 200);
        assert_eq!(done["status"], serde_json::json!([409]));
        let request = direct.prepare(&[&newer]).unwrap();
        assert_eq!(direct.apply(request).await.unwrap().len(), 1);
    }
}

#[path = "support/delay_proxy.rs"]
mod delay_proxy;

#[tokio::test(flavor = "current_thread")]
async fn empty_indexed_fields_preserve_exact_selected_values() {
    let (mut target, mapping) = native_fixture::fixture_with_fields(Vec::new());
    target.create_generation().await.unwrap();
    target.verify_generation().await.unwrap();
    let a = document(&mapping, 1, Some("stored only"));
    assert_eq!(
        target
            .apply(target.prepare(&[&a]).unwrap())
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        target
            .inspect_prepared(&target.prepare(&[&a]).unwrap())
            .await
            .unwrap()
            .len(),
        1
    );
}
