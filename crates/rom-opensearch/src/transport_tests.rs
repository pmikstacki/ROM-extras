//! Actual TLS sockets with controlled hostile responses; separate from native provider conformance.
use crate::{TlsConfig, transport::Transport};
use reqwest::Method;
use rom_projection_core::TargetFailure;
use std::{
    fs,
    os::unix::fs::DirBuilderExt,
    process::Command,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
struct Fixture {
    process: Option<fixture_process::FixtureProcess>,
    control: std::path::PathBuf,
    endpoint: String,
}
impl Fixture {
    fn new(mode: &str) -> Self {
        let control = std::env::temp_dir().join(format!(
            "rom-os-http-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::DirBuilder::new().mode(0o700).create(&control).unwrap();
        let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/support/hostile_https.mjs");
        let mut command = Command::new("node");
        command
            .arg(script)
            .arg("/root/ROM-extras/.superpowers/opensearch-fixture")
            .arg(mode)
            .arg(&control);
        let (process, port) =
            fixture_process::FixtureProcess::start(&mut command, Duration::from_secs(5)).unwrap();
        Self {
            process: Some(process),
            control,
            endpoint: format!("https://127.0.0.1:{port}"),
        }
    }
    fn transport(&self, wrong_ca: bool) -> Transport {
        let root = "/root/ROM-extras/.superpowers/opensearch-fixture";
        let ca = if wrong_ca { "projection-writer" } else { "ca" };
        Transport::new(
            TlsConfig::new(
                &self.endpoint,
                fs::read(format!("{root}/tls/{ca}.pem")).unwrap(),
                [
                    fs::read(format!("{root}/tls/projection-writer.pem")).unwrap(),
                    fs::read(format!("{root}/client-private/projection-writer.key")).unwrap(),
                ]
                .concat(),
                Duration::from_millis(300),
            )
            .unwrap(),
        )
        .unwrap()
    }
    fn counts(&self) -> serde_json::Value {
        serde_json::from_slice(&fs::read(self.control.join("counts")).unwrap()).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        drop(self.process.take());
        fs::remove_dir_all(&self.control).unwrap();
    }
}
#[tokio::test(flavor = "current_thread")]
async fn redirects_never_reach_destination_and_are_not_retried() {
    for status in [301, 302, 303, 307, 308] {
        let f = Fixture::new(&format!("redirect{status}"));
        assert!(matches!(
            f.transport(false)
                .request(Method::POST, &["private"], &[], Some(b"{}".to_vec()), false)
                .await,
            Err(TargetFailure::Unknown)
        ));
        assert_eq!(f.counts(), serde_json::json!({"source":1,"target":0}));
    }
}
#[tokio::test(flavor = "current_thread")]
async fn oversized_stream_declared_size_and_invalid_json_never_become_observations() {
    for mode in ["declared-large", "stream-large", "malformed"] {
        let f = Fixture::new(mode);
        assert!(matches!(
            f.transport(false)
                .request(Method::GET, &[], &[], None, false)
                .await,
            Err(TargetFailure::Unknown)
        ));
        assert_eq!(f.counts()["source"], 1);
    }
}
#[tokio::test(flavor = "current_thread")]
async fn operation_deadline_stops_a_server_that_never_responds() {
    let f = Fixture::new("deadline");
    let start = std::time::Instant::now();
    assert!(matches!(
        f.transport(false)
            .request(Method::GET, &[], &[], None, false)
            .await,
        Err(TargetFailure::Unknown)
    ));
    assert!(start.elapsed() < Duration::from_secs(2));
    assert_eq!(f.counts()["source"], 1);
}
#[tokio::test(flavor = "current_thread")]
async fn wrong_trust_root_prevents_any_http_disclosure() {
    let f = Fixture::new("ok");
    assert!(matches!(
        f.transport(true)
            .request(Method::POST, &[], &[], Some(b"{}".to_vec()), false)
            .await,
        Err(TargetFailure::Unknown)
    ));
    assert_eq!(f.counts(), serde_json::json!({"source":0,"target":0}));
}
#[tokio::test(flavor = "current_thread")]
async fn generation_inspection_has_one_total_deadline_across_successful_requests() {
    let f = Fixture::new("slow-generation");
    let profile =
        rom_projection_core::ProjectionProfile::new("fixture", "opensearch", "mapping", None)
            .unwrap();
    let mut target = crate::OpenSearch {
        transport: f.transport(false),
        profile: profile.clone(),
        physical: "fixture_generation".into(),
        profile_digest: crate::writes::hex(&profile.fingerprint()),
        text_fields: vec!["title".into()],
        index_uuid: None,
    };
    fs::write(
        f.control.join("mapping.json"),
        serde_json::to_vec(&crate::mapping::definition(
            &target.profile_digest,
            &target.text_fields,
        ))
        .unwrap(),
    )
    .unwrap();
    assert!(matches!(
        target.verify_generation().await,
        Err(TargetFailure::Unknown)
    ));
    assert!(matches!(
        target.create_generation().await,
        Err(TargetFailure::Unknown)
    ));
}

#[path = "../tests/support/fixture_process.rs"]
mod fixture_process;
#[test]
fn fixture_startup_timeout_and_malformed_readiness_join_owned_child() {
    for readiness in [
        "",
        "console.log('invalid readiness');",
        "console.log('12345678901234567890');",
        "console.log('12345');",
    ] {
        let control = std::env::temp_dir().join(format!(
            "rom-os-start-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::DirBuilder::new().mode(0o700).create(&control).unwrap();
        let pid_path = control.join("pid");
        let script = format!(
            "require('node:fs').writeFileSync(process.argv[1],String(process.pid));{readiness}setInterval(()=>{{}},1000);"
        );
        let mut command = Command::new("node");
        command.arg("-e").arg(script).arg(&pid_path);
        let start = std::time::Instant::now();
        let result =
            fixture_process::FixtureProcess::start(&mut command, Duration::from_millis(500));
        if readiness == "console.log('12345');" {
            let (owner, port) = result.unwrap();
            assert_eq!(port, 12345);
            drop(owner);
        } else {
            assert!(result.is_err());
        }
        assert!(start.elapsed() < Duration::from_secs(2));
        let pid = fs::read_to_string(pid_path).unwrap();
        assert!(!std::path::Path::new("/proc").join(pid).exists());
        fs::remove_dir_all(control).unwrap();
    }
}
