//! Authored real TLS/HTTP scenarios, distinct from native Nginx qualification.
use super::{context, plan, source};
use rom_import_transport::{Error, HttpConfig, HttpSource, StrongEtag};
use std::{sync::Arc, time::Duration};
pub async fn run(endpoint: &str, ca: &[u8], token: &str) {
    let etag = StrongEtag::new("\"v1\"").unwrap();
    for (case, expected) in [
        ("missing", Error::Missing),
        ("precondition", Error::PreconditionFailed),
        (
            "rate",
            Error::RateLimited {
                retry_after_seconds: Some(3),
            },
        ),
        (
            "rate-invalid",
            Error::RateLimited {
                retry_after_seconds: None,
            },
        ),
        ("unavailable", Error::Unavailable),
        ("unauthorized", Error::Rejected),
        ("redirect", Error::Rejected),
        ("no-content", Error::Rejected),
        ("partial", Error::Rejected),
        ("not-modified", Error::Rejected),
        ("encoded", Error::Rejected),
        ("mime", Error::Rejected),
        ("missing-etag", Error::PreconditionFailed),
        ("wrong-etag", Error::PreconditionFailed),
        ("duplicate-etag", Error::PreconditionFailed),
        ("declared-large", Error::TooLarge),
        ("chunked-large", Error::TooLarge),
        ("truncated", Error::Unavailable),
        ("headers-count", Error::Unavailable),
        ("headers-bytes", Error::TooLarge),
        (
            "changed",
            Error::Admission(rom_import::Error::DigestMismatch),
        ),
    ] {
        let (_sender, ctx) = context(2000);
        let result = source(&format!("{endpoint}/{case}"), ca, token, Duration::ZERO)
            .fetch(&plan(b"21"), Some(&etag), &ctx)
            .await;
        assert_eq!(result.unwrap_err(), expected, "case {case}");
    }
    for (case, body) in [
        ("duplicate-json", br#"{"k":1,"k":2}"#.as_slice()),
        ("invalid-utf8", &[255]),
    ] {
        let (_sender, ctx) = context(2000);
        assert_eq!(
            source(&format!("{endpoint}/{case}"), ca, token, Duration::ZERO)
                .fetch(&plan(body), Some(&etag), &ctx)
                .await
                .unwrap_err(),
            Error::Admission(rom_import::Error::InvalidJson)
        );
    }
    for case in ["good", "chunked", "charset"] {
        let (_sender, ctx) = context(2000);
        let prepared = source(&format!("{endpoint}/{case}"), ca, token, Duration::ZERO)
            .fetch(&plan(b"21"), Some(&etag), &ctx)
            .await
            .unwrap();
        let (request, _) = prepared.request();
        assert_eq!(request.id, "exact[0].id");
        let rom::Operation::Action { input, .. } = request.operation else {
            panic!("action required")
        };
        assert_eq!(input.as_u64(), Some(21));
    }
    let (_sender, ctx) = context(2000);
    let no_trust = HttpSource::new(
        HttpConfig::new(&format!("{endpoint}/untrusted"), "host", Duration::ZERO, 1).unwrap(),
    )
    .unwrap();
    assert_eq!(
        no_trust.fetch(&plan(b"21"), None, &ctx).await.unwrap_err(),
        Error::Unavailable
    );
    for case in ["hold-headers", "hold-body"] {
        let (_sender, ctx) = context(60);
        assert_eq!(
            source(&format!("{endpoint}/{case}"), ca, token, Duration::ZERO)
                .fetch(&plan(b"21"), Some(&etag), &ctx)
                .await
                .unwrap_err(),
            Error::Timeout
        );
    }
    let (sender, ctx) = context(2000);
    let send = sender.clone();
    let cancelled = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(40)).await;
        send.send_replace(true);
    });
    assert_eq!(
        source(&format!("{endpoint}/cancel"), ca, token, Duration::ZERO)
            .fetch(&plan(b"21"), Some(&etag), &ctx)
            .await
            .unwrap_err(),
        Error::Cancelled
    );
    cancelled.await.unwrap();
    let (sender, ctx) = context(2000);
    sender.send_replace(true);
    assert_eq!(
        source(&format!("{endpoint}/pre-cancel"), ca, token, Duration::ZERO)
            .fetch(&plan(b"21"), None, &ctx)
            .await
            .unwrap_err(),
        Error::Cancelled
    );
    let (sender, ctx) = context(2000);
    drop(sender);
    assert_eq!(
        source(
            &format!("{endpoint}/closed-cancel"),
            ca,
            token,
            Duration::ZERO
        )
        .fetch(&plan(b"21"), None, &ctx)
        .await
        .unwrap_err(),
        Error::Cancelled
    );
    let pacing = source(
        &format!("{endpoint}/paced"),
        ca,
        token,
        Duration::from_millis(300),
    );
    let (_sender, ctx) = context(2000);
    pacing.fetch(&plan(b"21"), None, &ctx).await.unwrap();
    let (_sender, ctx) = context(20);
    assert_eq!(
        pacing.fetch(&plan(b"21"), None, &ctx).await.unwrap_err(),
        Error::Timeout
    );
    let shared = Arc::new(source(
        &format!("{endpoint}/concurrency"),
        ca,
        token,
        Duration::ZERO,
    ));
    let first = shared.clone();
    let pending = tokio::spawn(async move {
        let (_sender, ctx) = context(150);
        first.fetch(&plan(b"21"), None, &ctx).await
    });
    tokio::task::yield_now().await;
    let (_sender, ctx) = context(2000);
    assert_eq!(
        shared.fetch(&plan(b"21"), None, &ctx).await.unwrap_err(),
        Error::Overloaded
    );
    assert_eq!(pending.await.unwrap().unwrap_err(), Error::Timeout);
    println!(
        "authored HTTPS: exact bytes, validators, status/size/header limits, credentials, TLS, pacing, cancellation and concurrency passed"
    );
}
