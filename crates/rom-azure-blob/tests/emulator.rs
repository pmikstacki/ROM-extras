//! Actual Azure protocol emulator tests; these do not establish cloud Azure support.
use rom_azure_blob::{AzureBlob, AzureConfig, Credentials, EndpointPolicy, Limits};
use rom_blob::{BlobStore, Digest, Error, ObjectKey};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
#[path = "../../../tests/common/azure_fixture.rs"]
mod fixture;
use fixture::emulator as store;
static FIXTURE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
#[tokio::test]
async fn published_blob_contract_passes_against_azurite() {
    let _exclusive_fixture = FIXTURE.lock().await;
    rom_conformance::blob::basic(&store(16)).await.unwrap();
}
#[tokio::test]
async fn false_payload_retains_metadata_and_bytes_after_emulator_restart() {
    let _exclusive_fixture = FIXTURE.lock().await;
    let adapter = store(1024);
    let identity = format!(
        "restart-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let key = ObjectKey::parse(Digest::of(identity.as_bytes()).as_str()).unwrap();
    adapter.create(&key, b"false".to_vec()).await.unwrap();
    assert_eq!(adapter.head(&key).await.unwrap().bytes, 5);
    let label = std::process::Command::new("docker")
        .args([
            "inspect",
            "rom-extras-azurite-20261008",
            "--format",
            "{{index .Config.Labels \"rom-extras.fixture\"}}",
        ])
        .output()
        .unwrap();
    assert!(label.status.success());
    assert_eq!(String::from_utf8(label.stdout).unwrap().trim(), "azurite");
    assert!(
        tokio::task::spawn_blocking(|| std::process::Command::new("docker")
            .args(["restart", "--time", "10", "rom-extras-azurite-20261008"])
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap()
            .success())
        .await
        .unwrap()
    );
    let rebound = store(1024);
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            match rebound.head(&key).await {
                Ok(meta) => {
                    assert_eq!(meta.bytes, 5);
                    break;
                }
                Err(Error::Backend | Error::Timeout) => {
                    tokio::time::sleep(Duration::from_millis(100)).await
                }
                other => panic!("unexpected post-restart metadata outcome: {other:?}"),
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(rebound.get(&key, 5).await.unwrap(), b"false");
    assert_eq!(rebound.get(&key, 4).await, Err(Error::TooLarge));
    // Preserve this acknowledged row for later recovery evidence.
}

#[tokio::test]
async fn wrong_account_key_is_confirmed_denied() {
    let _exclusive_fixture = FIXTURE.lock().await;
    let endpoint = std::env::var("ROM_EXTRAS_AZURE_ENDPOINT").unwrap();
    assert_eq!(endpoint, "http://127.0.0.1:55454/fixtureaccount");
    let wrong_key = format!("{}==", "A".repeat(86));
    let adapter = AzureBlob::connect(
        AzureConfig {
            account: "fixtureaccount",
            container: "rom-extras",
            endpoint: &endpoint,
            policy: EndpointPolicy::LoopbackEmulator,
            credentials: Credentials::SharedKey(&wrong_key),
        },
        Limits::default(),
    )
    .unwrap();
    let key = ObjectKey::parse(Digest::of(b"denied-false").as_str()).unwrap();
    assert_eq!(
        adapter.create(&key, b"false".to_vec()).await,
        Err(Error::Denied)
    );
    assert_eq!(adapter.head(&key).await, Err(Error::Denied));
    assert_eq!(store(1024).head(&key).await, Err(Error::Missing));
}
