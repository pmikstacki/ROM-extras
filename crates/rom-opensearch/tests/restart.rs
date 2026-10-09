//! Persistent native server restart; inspect state before any replay can recreate lost data.
#![cfg(feature = "service-fixture")]
#[path = "support/approved_document.rs"]
mod approved_document;
#[path = "support/native_fixture.rs"]
mod native_fixture;
use rom_projection_core::{ProjectionTarget, TargetFailure};
#[test]
fn restart_preserves_exact_live_and_tombstone_before_replay() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let (mut target, mapping) = native_fixture::fixture();
    rt.block_on(native_fixture::create_generation(&mut target));
    let live =
        approved_document::document(&mapping, i64::MAX as u64, Some("persisted signed maximum"));
    let request = target.prepare(&[&live]).unwrap();
    rt.block_on(target.apply(request)).unwrap();
    let tomb = mapping
        .document(
            &rom::JournalView {
                position: 2,
                view: rom::ProjectedView {
                    key: rom::Key {
                        kind: "native_docs".into(),
                        id: "persistent-tombstone".into(),
                    },
                    revision: 2,
                    value: None,
                },
            },
            None,
        )
        .unwrap();
    let request = target.prepare(&[&tomb]).unwrap();
    rt.block_on(target.apply(request)).unwrap();
    let inspection = target.prepare(&[&live, &tomb]).unwrap();
    let result = std::process::Command::new("timeout")
        .args([
            "--kill-after=2s",
            "30s",
            "docker",
            "restart",
            "--time",
            "10",
            "rom-extras-opensearch-20261008",
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(result.success());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(40);
    loop {
        match rt.block_on(target.inspect_prepared(&inspection)) {
            Ok(observed) => {
                assert_eq!(observed.len(), 2);
                break;
            }
            Err(error) => {
                assert!(
                    std::time::Instant::now() < deadline,
                    "native restart inspection failed: {error:?}"
                );
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }
    let old = mapping
        .document(
            &rom::JournalView {
                position: 1,
                view: rom::ProjectedView {
                    key: rom::Key {
                        kind: "native_docs".into(),
                        id: "persistent-tombstone".into(),
                    },
                    revision: 1,
                    value: Some(
                        serde_json::json!({"title":"late resurrection"})
                            .as_object()
                            .unwrap()
                            .clone(),
                    ),
                },
            },
            None,
        )
        .unwrap();
    let request = target.prepare(&[&old]).unwrap();
    assert!(matches!(
        rt.block_on(target.apply(request)),
        Err(TargetFailure::Rejected)
    ));
    assert_eq!(
        rt.block_on(target.inspect_prepared(&inspection))
            .unwrap()
            .len(),
        2
    );
}
