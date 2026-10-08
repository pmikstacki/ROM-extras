//! Control only the two dedicated, labelled RabbitMQ fixtures.
use std::time::Duration;
pub async fn restart(container: &'static str, expected_label: &'static str) {
    assert!(
        matches!(
            (container, expected_label),
            ("rom-extras-rabbitmq-20261008", "rabbitmq")
                | ("rom-extras-rabbitmq-tls-20261008", "rabbitmq-tls")
        ),
        "dedicated fixture required"
    );
    let label = std::process::Command::new("docker")
        .args([
            "inspect",
            container,
            "--format",
            "{{index .Config.Labels \"rom-extras.fixture\"}}",
        ])
        .output()
        .unwrap();
    assert!(label.status.success());
    assert_eq!(
        String::from_utf8(label.stdout).unwrap().trim(),
        expected_label
    );
    assert!(
        tokio::task::spawn_blocking(move || std::process::Command::new("docker")
            .args(["restart", "--time", "10", container])
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap()
            .success())
        .await
        .unwrap()
    );
    // RabbitMQ starts asynchronously after the container process starts.
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            let ready = tokio::task::spawn_blocking(move || {
                std::process::Command::new("docker")
                    .args([
                        "exec",
                        container,
                        "rabbitmq-diagnostics",
                        "-q",
                        "check_running",
                    ])
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                    .unwrap()
                    .success()
            })
            .await
            .unwrap();
            if ready {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .unwrap();
}
