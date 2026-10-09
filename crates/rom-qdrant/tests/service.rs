//! Actual Rust-generated request acceptance; never skip a missing native fixture.
#![cfg(feature = "service-fixture")]
use rom::{JournalView, Key, ProjectedView, json};
use rom_projection_core::{DocumentMapping, ProjectionProfile};
use rom_qdrant::PreparedWrite;
use std::{
    io::Write,
    process::{Command, Stdio},
};
#[test]
fn native_original_key_collision_and_max_revision_tombstone_are_fenced() {
    let mapping = DocumentMapping::new(
        ProjectionProfile::new("deployment", "qdrant", "mapping", Some("model")).unwrap(),
        vec!["value".into()],
        Some(3),
    )
    .unwrap();
    let mut requests = Vec::new();
    for (key, revision, tombstone, value) in [
        ("original", 1, false, u64::MAX),
        ("collision", u64::MAX, false, u64::MAX),
        ("original", 1_u64 << 32, false, u64::MAX),
        ("original", u64::MAX, true, u64::MAX),
        ("original", 1, false, 7),
    ] {
        let doc = mapping
            .document(
                &JournalView {
                    position: 1,
                    view: ProjectedView {
                        key: Key {
                            kind: "documents".into(),
                            id: key.into(),
                        },
                        revision,
                        value: (!tombstone)
                            .then(|| json!({"value":value}).as_object().unwrap().clone()),
                    },
                },
                (!tombstone).then(|| vec![1.0, 0.0, 0.0]),
            )
            .unwrap();
        requests.push(
            serde_json::from_slice::<serde_json::Value>(
                PreparedWrite::new(&doc).unwrap().as_bytes(),
            )
            .unwrap(),
        );
    }
    let python = std::env::var("ROM_EXTRAS_PYTHON").expect("required fixture Python");
    let mut child = Command::new("timeout")
        .args(["--kill-after=2s", "45s", &python])
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/native_fence.py"
        ))
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&requests).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "native fixture failed (sanitized exit): {:?}",
        output.status.code()
    );
    let receipt: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(receipt["passed"], true);
    println!("{}", String::from_utf8(output.stdout).unwrap());
}
