//! Actual HTTP200 partial failure and durable source inspection with malformed native wire.
use rom_projection_core::{ProjectionTarget, TargetFailure};
#[path = "../tests/support/approved_document.rs"]
mod approved_document;
#[path = "../tests/support/native_fixture.rs"]
mod native_fixture;
use approved_document::document;
use native_fixture::fixture;
#[tokio::test(flavor = "current_thread")]
async fn http200_partial_bulk_cannot_publish_complete_observations() {
    let (mut target, mapping) = fixture();
    native_fixture::create_generation(&mut target).await;
    let a = document(&mapping, 1, Some("approved"));
    let b = mapping
        .document(
            &rom::JournalView {
                position: 2,
                view: rom::ProjectedView {
                    key: rom::Key {
                        kind: "native_docs".into(),
                        id: "second".into(),
                    },
                    revision: 1,
                    value: Some(
                        serde_json::json!({"title":"second"})
                            .as_object()
                            .unwrap()
                            .clone(),
                    ),
                },
            },
            None,
        )
        .unwrap();
    let mut request = target.prepare(&[&a, &b]).unwrap();
    let mut lines: Vec<serde_json::Value> = request
        .body
        .split(|b| *b == b'\n')
        .filter(|s| !s.is_empty())
        .map(|s| serde_json::from_slice(s).unwrap())
        .collect();
    lines[3]["rom_revision"] = serde_json::json!("invalid-number");
    request.body = lines
        .iter()
        .flat_map(|v| {
            let mut b = serde_json::to_vec(v).unwrap();
            b.push(b'\n');
            b
        })
        .collect();
    assert!(matches!(
        target.apply(request).await,
        Err(TargetFailure::Rejected)
    ));
    let first_id = lines[0]["index"]["_id"].as_str().unwrap();
    let (_, stored) = target
        .transport
        .request(
            reqwest::Method::GET,
            &[target.physical_target(), "_doc", first_id],
            &[],
            None,
            false,
        )
        .await
        .unwrap();
    assert_eq!(stored["_source"], lines[1]);
    let wire: Vec<u8> = lines
        .iter()
        .flat_map(|v| {
            let mut b = serde_json::to_vec(v).unwrap();
            b.push(b'\n');
            b
        })
        .collect();
    let (status, partial) = target
        .transport
        .request(
            reqwest::Method::POST,
            &[target.physical_target(), "_bulk"],
            &[],
            Some(wire),
            true,
        )
        .await
        .unwrap();
    assert_eq!(status, 200);
    assert_eq!(partial["errors"], true);
    assert_eq!(partial["items"][0]["index"]["status"], 409);
    assert_eq!(partial["items"][1]["index"]["status"], 400);
    // The successful item remains native state despite the failed page.
    let request = target.prepare(&[&a]).unwrap();
    assert_eq!(target.apply(request).await.unwrap().len(), 1);
}
