//! Import acquisition admission through public APIs.
use rom::{Key, RevisionCondition};
use rom_import::{ActionBinding, ActionPlan, Limits, SourceGrant};
use rom_import_transport::{Error, prepare_file};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{Seek, SeekFrom, Write},
    path::PathBuf,
};
fn plan(bytes: &[u8], limit: usize) -> ActionPlan {
    ActionPlan::trusted(
        ActionBinding {
            target: Key {
                kind: "values".into(),
                id: "exact[0].id".into(),
            },
            action: "import".into(),
            expected: Some(1),
            idempotency: "original".into(),
            retry_epoch: 0,
        },
        SourceGrant {
            source: "deployment".into(),
            generation: 1,
            output_origins: BTreeMap::from([("value".into(), "host".into())]),
            condition: RevisionCondition {
                key: Key {
                    kind: "controls".into(),
                    id: "source".into(),
                },
                revision: 1,
            },
            valid_until: u64::MAX,
            expected_sha256: Sha256::digest(bytes).into(),
        },
        Limits::new(limit, 32, 4096, 16384).unwrap(),
    )
    .unwrap()
}
fn fixture() -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../.superpowers/import-files-{}-{stamp}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).unwrap();
    root
}
fn input(bytes: &[u8]) -> File {
    let path = fixture().join("private-input.json");
    let mut f = std::fs::OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(path)
        .unwrap();
    f.write_all(bytes).unwrap();
    f
}
#[test]
fn file_cursor_cannot_change_the_approved_representation() {
    let mut file = input(b"21");
    file.seek(SeekFrom::Start(1)).unwrap();
    let prepared = prepare_file(&plan(b"21", 2), file).unwrap();
    let (request, _) = prepared.request();
    let rom::Operation::Action { input, .. } = request.operation else {
        panic!("action required")
    };
    assert_eq!(input.as_u64(), Some(21));
    assert_eq!(request.id, "exact[0].id");
}
#[test]
fn file_limits_and_changed_truncated_or_invalid_input_fail_closed() {
    assert_eq!(
        prepare_file(&plan(b"21", 1), input(b"21")).unwrap_err(),
        Error::TooLarge
    );
    for bytes in [b"22".as_slice(), b"2", b"21 "] {
        assert_eq!(
            prepare_file(&plan(b"21", 100), input(bytes)).unwrap_err(),
            Error::Admission(rom_import::Error::DigestMismatch)
        );
    }
    let invalid = [255];
    assert_eq!(
        prepare_file(&plan(&invalid, 100), input(&invalid)).unwrap_err(),
        Error::Admission(rom_import::Error::InvalidJson)
    );
}
#[cfg(unix)]
#[test]
fn nonregular_handles_never_become_import_documents() {
    let p = plan(b"21", 100);
    for file in [
        File::open(fixture()).unwrap(),
        File::open("/dev/null").unwrap(),
    ] {
        assert_eq!(
            prepare_file(&p, file).unwrap_err(),
            Error::UnsupportedSource
        );
    }
    let (socket, _peer) = std::os::unix::net::UnixStream::pair().unwrap();
    let fd: std::os::fd::OwnedFd = socket.into();
    assert_eq!(
        prepare_file(&p, File::from(fd)).unwrap_err(),
        Error::UnsupportedSource
    );
}
#[cfg(feature = "http")]
#[test]
fn endpoint_validator_and_credential_diagnostics_are_explicit_and_private() {
    use rom_import_transport::{HttpConfig, HttpSource, StrongEtag};
    use std::time::Duration;
    for endpoint in [
        "http://example.test/input",
        "https://u:SECRET@example.test/input",
        "https://example.test/input?key=SECRET",
        "https://example.test/input#x",
        "https://example.test/a/../b",
        "https://example.test/%2e%2e/b",
    ] {
        assert!(HttpConfig::new(endpoint, "host", Duration::ZERO, 1).is_err());
    }
    for etag in [
        "*",
        "W/\"v1\"",
        "\"one\", \"two\"",
        "bad",
        "\"x\r\nSECRET\"",
    ] {
        assert!(StrongEtag::new(etag).is_err());
    }
    let config = HttpConfig::new(
        "https://example.test/private-source",
        "host",
        Duration::ZERO,
        1,
    )
    .unwrap()
    .with_bearer("PRIVATE-TOKEN".into())
    .unwrap();
    let text = format!("{config:?}");
    assert!(!text.contains("PRIVATE"));
    assert!(!text.contains("private-source"));
    let source = HttpSource::new(config).unwrap();
    assert!(!format!("{source:?}").contains("PRIVATE"));
    assert!(!format!("{:?}", StrongEtag::new("\"PRIVATE\"").unwrap()).contains("PRIVATE"));
}

#[cfg(feature = "http")]
#[test]
fn terminal_dot_segments_cannot_change_the_host_selected_path() {
    use rom_import_transport::HttpConfig;
    for endpoint in [
        "https://example.test/approved/..",
        "https://example.test/approved/.",
    ] {
        assert!(HttpConfig::new(endpoint, "host", std::time::Duration::ZERO, 1).is_err());
    }
}
