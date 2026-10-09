//! Test setup only: metadata existence does not establish shard readiness.
use serde_json::{Value, json};
use std::{fs, time::Duration};

pub(super) async fn wait(physical: &str) {
    let root = "/root/ROM-extras/.superpowers/opensearch-fixture";
    let client = super::fixture_admin::client();
    let started = std::time::Instant::now();
    let mut first = None;
    let mut last = Value::Null;
    let inspection = async {
        loop {
            match client
                .get(format!(
                    "https://127.0.0.1:55460/_cluster/health/{physical}?level=indices&wait_for_status=green&timeout=1s"
                ))
                .send()
                .await
            {
                Ok(mut response) if response.status().is_success() || response.status().as_u16() == 408 => {
                    assert!(
                        response
                            .content_length()
                            .is_none_or(|size| size <= 1_048_576)
                    );
                    let mut bytes = Vec::new();
                    while let Some(chunk) = response.chunk().await.unwrap() {
                        assert!(bytes.len() + chunk.len() <= 1_048_576);
                        bytes.extend_from_slice(&chunk);
                    }
                    let body: Value = serde_json::from_slice(&bytes).unwrap();
                    let index = &body["indices"][physical];
                    last = json!({"timed_out": body["timed_out"], "status": index["status"],
                        "active_primary_shards": index["active_primary_shards"],
                        "initializing_shards": index["initializing_shards"],
                        "relocating_shards": index["relocating_shards"],
                        "unassigned_shards": index["unassigned_shards"]});
                    first.get_or_insert_with(|| last.clone());
                    if last["timed_out"] == false
                        && last["status"] == "green"
                        && last["active_primary_shards"] == 1
                        && last["initializing_shards"] == 0
                        && last["relocating_shards"] == 0
                        && last["unassigned_shards"] == 0
                    {
                        break;
                    }
                }
                Ok(response) if matches!(response.status().as_u16(), 429 | 503) => {}
                Ok(response) => panic!("native readiness HTTP {}", response.status()),
                Err(_) => {}
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    };
    let result = tokio::time::timeout(Duration::from_secs(45), inspection).await;
    let evidence = json!({"physical": physical, "elapsed_ms": started.elapsed().as_millis(),
        "first": first, "last": last, "ready": result.is_ok()});
    fs::write(
        format!("{root}/readiness-{physical}.json"),
        serde_json::to_vec(&evidence).unwrap(),
    )
    .unwrap();
    result.expect("native primary shard did not become ready within setup deadline");
}
