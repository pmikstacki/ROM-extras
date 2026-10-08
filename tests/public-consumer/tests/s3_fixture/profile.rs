use rom_blob_object_store::{Adapter, EndpointPolicy, S3Config};

struct Fixture {
    endpoint: &'static str,
    container: &'static str,
    label: &'static str,
}

fn fixture() -> Fixture {
    match std::env::var("ROM_EXTRAS_S3_PROFILE").as_deref() {
        Ok("seaweedfs") | Err(std::env::VarError::NotPresent) => Fixture {
            endpoint: "http://127.0.0.1:55455",
            container: "rom-extras-seaweedfs-20261008",
            label: "seaweedfs",
        },
        Ok("rustfs") => Fixture {
            endpoint: "http://127.0.0.1:55456",
            container: "rom-extras-rustfs-20261008",
            label: "rustfs",
        },
        _ => panic!("unsupported S3 fixture profile"),
    }
}

pub(crate) fn adapter(max_bytes: usize, wrong_secret: bool) -> Adapter {
    let selected = fixture();
    let endpoint = std::env::var("ROM_EXTRAS_S3_ENDPOINT").expect("required S3 fixture endpoint");
    let bucket = std::env::var("ROM_EXTRAS_S3_BUCKET").expect("required S3 fixture bucket");
    let region = std::env::var("ROM_EXTRAS_S3_REGION").expect("required S3 fixture region");
    let access = std::env::var("ROM_EXTRAS_S3_ACCESS_KEY").expect("private S3 fixture key");
    let secret = std::env::var("ROM_EXTRAS_S3_SECRET").expect("private S3 fixture secret");
    assert_eq!(endpoint, selected.endpoint);
    assert_eq!(bucket, "rom-extras");
    assert_eq!(region, "us-east-1");
    Adapter::s3(
        S3Config {
            endpoint: &endpoint,
            region: &region,
            bucket: &bucket,
            access_key: &access,
            secret: if wrong_secret {
                "deliberately-invalid-fixture-secret"
            } else {
                &secret
            },
            policy: EndpointPolicy::LoopbackTestOnly,
        },
        max_bytes,
    )
    .unwrap()
}

pub(crate) async fn restart() {
    let selected = fixture();
    let label = std::process::Command::new("docker")
        .args([
            "inspect",
            selected.container,
            "--format",
            "{{index .Config.Labels \"rom-extras.fixture\"}}",
        ])
        .output()
        .unwrap();
    assert!(label.status.success());
    assert_eq!(
        String::from_utf8(label.stdout).unwrap().trim(),
        selected.label
    );
    assert!(
        tokio::task::spawn_blocking(move || std::process::Command::new("docker")
            .args(["restart", "--time", "10", selected.container])
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap()
            .success())
        .await
        .unwrap()
    );
}
