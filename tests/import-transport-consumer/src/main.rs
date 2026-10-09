//! Independent acquisition consumer; fixtures are host-selected, never library defaults.
#[allow(
    dead_code,
    reason = "Shared fixture also serves the core-only consumer"
)]
#[path = "../../import-public-consumer/src/model.rs"]
mod model;
mod native;
mod protocol;
use rom::{Key, RevisionCondition};
use rom_import::{ActionBinding, ActionPlan, Limits, SourceGrant};
use rom_import_transport::{HttpConfig, HttpSource, RequestContext};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, time::Duration};
fn plan(bytes: &[u8]) -> ActionPlan {
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
        Limits::default(),
    )
    .unwrap()
}
fn source(endpoint: &str, ca: &[u8], token: &str, interval: Duration) -> HttpSource {
    HttpSource::new(
        HttpConfig::new(endpoint, "ROM-extras independent host", interval, 1)
            .unwrap()
            .with_ca(ca.to_vec())
            .unwrap()
            .with_bearer(token.into())
            .unwrap(),
    )
    .unwrap()
}
fn context(ms: u64) -> (tokio::sync::watch::Sender<bool>, RequestContext) {
    let (sender, receiver) = tokio::sync::watch::channel(false);
    (
        sender,
        RequestContext::new(Duration::from_millis(ms), receiver).unwrap(),
    )
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args
        .get(1)
        .is_some_and(|mode| mode.starts_with("--native-"))
    {
        native::dispatch(&args);
        return;
    }
    assert_eq!(
        args.len(),
        4,
        "explicit endpoint, CA and private credential file required"
    );
    let ca = std::fs::read(&args[2]).unwrap();
    let bytes = std::fs::read(&args[3]).unwrap();
    assert!(bytes.len() < 8192);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&args[3]).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    let config: serde_json::Value = rom::parse_json(&bytes).unwrap();
    let token = config["token"].as_str().unwrap();
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .enable_io()
        .build()
        .unwrap()
        .block_on(protocol::run(&args[1], &ca, token));
}
