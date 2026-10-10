//! Public API control-flow evidence; stores here are in-memory fixtures.
use rom_blob::{BlobStore, Digest, Metadata, ObjectKey, StoreFuture};
use rom_blob_recovery::{Cause, Context, Entry, Limits, Manifest, Publication, copy, verify};
use std::{
    collections::BTreeMap,
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio_util::sync::CancellationToken;

#[derive(Default)]
struct Store {
    objects: Mutex<BTreeMap<String, Vec<u8>>>,
    writes: AtomicUsize,
    unknown: bool,
}
impl BlobStore for Store {
    fn create<'a>(&'a self, k: &'a ObjectKey, b: Vec<u8>) -> StoreFuture<'a, ()> {
        Box::pin(async move {
            self.writes.fetch_add(1, Ordering::SeqCst);
            let mut objects = self.objects.lock().unwrap();
            if objects.contains_key(k.as_str()) {
                return Err(rom_blob::Error::Conflict);
            }
            objects.insert(k.as_str().into(), b);
            if self.unknown {
                Err(rom_blob::Error::Unknown)
            } else {
                Ok(())
            }
        })
    }
    fn get<'a>(&'a self, k: &'a ObjectKey, max: usize) -> StoreFuture<'a, Vec<u8>> {
        Box::pin(async move {
            let b = self
                .objects
                .lock()
                .unwrap()
                .get(k.as_str())
                .cloned()
                .ok_or(rom_blob::Error::Missing)?;
            if b.len() > max {
                return Err(rom_blob::Error::TooLarge);
            }
            Ok(b)
        })
    }
    fn head<'a>(&'a self, k: &'a ObjectKey) -> StoreFuture<'a, Metadata> {
        Box::pin(async move {
            Ok(Metadata {
                bytes: self.get(k, usize::MAX).await?.len() as u64,
            })
        })
    }
    fn delete<'a>(&'a self, _: &'a ObjectKey) -> StoreFuture<'a, ()> {
        panic!("recovery must never delete")
    }
}
fn manifest(bytes: &[u8]) -> Manifest {
    Manifest::new(
        Digest::of(b"approved-checkpoint"),
        vec![Entry::new(
            ObjectKey::parse(Digest::of(bytes).as_str()).unwrap(),
            Digest::of(bytes),
            bytes.len() as u64,
        )],
        Limits::default(),
    )
    .unwrap()
}
fn ctx() -> Context {
    Context::new(
        tokio::time::Instant::now() + Duration::from_secs(5),
        CancellationToken::new(),
    )
}
fn source(b: &[u8]) -> Store {
    let s = Store::default();
    s.objects
        .lock()
        .unwrap()
        .insert(Digest::of(b).as_str().into(), b.into());
    s
}
#[tokio::test(flavor = "current_thread")]
async fn copy_verifies_bytes_and_equal_resume_never_writes_again() {
    let src = source(b"payload");
    let dst = Store::default();
    let m = manifest(b"payload");
    let r = copy(&src, &dst, &m, &ctx()).await.unwrap();
    assert_eq!(r.verified, 1);
    assert_eq!(r.created, 1);
    let r = copy(&Store::default(), &dst, &m, &ctx()).await.unwrap();
    assert_eq!(r.existing, 1);
    assert_eq!(dst.writes.load(Ordering::SeqCst), 1);
    assert_eq!(verify(&dst, &m, &ctx()).await.unwrap().bytes, 7);
}
#[tokio::test(flavor = "current_thread")]
async fn corrupt_destination_is_refused_without_overwrite() {
    let dst = source(b"payload");
    dst.objects
        .lock()
        .unwrap()
        .insert(Digest::of(b"payload").as_str().into(), b"CORRUPT".into());
    let e = copy(&source(b"payload"), &dst, &manifest(b"payload"), &ctx())
        .await
        .unwrap_err();
    assert_eq!(e.cause, Cause::Integrity);
    assert_eq!(e.publication, Publication::NotAttempted);
    assert_eq!(dst.writes.load(Ordering::SeqCst), 0);
}
#[tokio::test(flavor = "current_thread")]
async fn unknown_stops_and_explicit_resume_reconciles_without_write() {
    let dst = Store {
        unknown: true,
        ..Store::default()
    };
    let m = manifest(b"payload");
    let e = copy(&source(b"payload"), &dst, &m, &ctx())
        .await
        .unwrap_err();
    assert_eq!(e.cause, Cause::Unknown);
    assert_eq!(e.publication, Publication::Unknown);
    assert_eq!(e.progress.verified, 0);
    assert_eq!(
        copy(&Store::default(), &dst, &m, &ctx())
            .await
            .unwrap()
            .existing,
        1
    );
    assert_eq!(dst.writes.load(Ordering::SeqCst), 1);
}
#[test]
fn rejects_duplicate_keys_and_bounded_wire_before_io() {
    let e = Entry::new(
        ObjectKey::parse(Digest::of(b"x").as_str()).unwrap(),
        Digest::of(b"x"),
        1,
    );
    assert!(
        Manifest::new(
            Digest::of(b"checkpoint"),
            vec![e.clone(), e],
            Limits::default()
        )
        .is_err()
    );
    let m = manifest(b"payload");
    let bytes = m.encode().unwrap();
    assert_eq!(
        Manifest::decode(&bytes, Limits::default())
            .unwrap()
            .entries()[0]
            .key()
            .as_str(),
        Digest::of(b"payload").as_str()
    );
    assert!(
        Manifest::decode(
            &bytes,
            Limits {
                max_json_bytes: 1,
                ..Limits::default()
            }
        )
        .is_err()
    );
    assert!(!format!("{m:?}").contains(Digest::of(b"payload").as_str()));
}

#[derive(Default)]
struct Faults {
    inner: Store,
    get_error: Option<rom_blob::Error>,
    stall_create: bool,
    concurrent_content: Option<Vec<u8>>,
    fail_after_create: bool,
    cancel_after_create: Option<CancellationToken>,
}
impl BlobStore for Faults {
    fn create<'a>(&'a self, k: &'a ObjectKey, b: Vec<u8>) -> StoreFuture<'a, ()> {
        Box::pin(async move {
            if let Some(bytes) = &self.concurrent_content {
                self.inner.create(k, bytes.clone()).await?;
                return Err(rom_blob::Error::Conflict);
            }
            if self.stall_create {
                self.inner.writes.fetch_add(1, Ordering::SeqCst);
                if let Some(cancel) = &self.cancel_after_create {
                    cancel.cancel();
                }
                return std::future::pending().await;
            }
            self.inner.create(k, b).await?;
            if let Some(c) = &self.cancel_after_create {
                c.cancel();
            }
            Ok(())
        })
    }
    fn get<'a>(&'a self, k: &'a ObjectKey, max: usize) -> StoreFuture<'a, Vec<u8>> {
        Box::pin(async move {
            if let Some(e) = &self.get_error {
                return Err(e.clone());
            }
            if self.fail_after_create && self.inner.writes.load(Ordering::SeqCst) > 0 {
                return Err(rom_blob::Error::Backend);
            }
            self.inner.get(k, max).await
        })
    }
    fn head<'a>(&'a self, k: &'a ObjectKey) -> StoreFuture<'a, Metadata> {
        self.inner.head(k)
    }
    fn delete<'a>(&'a self, _: &'a ObjectKey) -> StoreFuture<'a, ()> {
        panic!("never delete")
    }
}
#[tokio::test(flavor = "current_thread")]
async fn cancellation_before_admission_never_writes() {
    let cancel = CancellationToken::new();
    cancel.cancel();
    let dst = Store::default();
    let c = Context::new(tokio::time::Instant::now() + Duration::from_secs(1), cancel);
    let e = copy(&source(b"payload"), &dst, &manifest(b"payload"), &c)
        .await
        .unwrap_err();
    assert_eq!(e.cause, Cause::Canceled);
    assert_eq!(e.publication, Publication::NotAttempted);
    assert_eq!(dst.writes.load(Ordering::SeqCst), 0);
}
#[tokio::test(flavor = "current_thread")]
async fn publication_timeout_is_unknown_and_never_retried() {
    let dst = Faults {
        stall_create: true,
        ..Faults::default()
    };
    let c = Context::new(
        tokio::time::Instant::now() + Duration::from_millis(20),
        CancellationToken::new(),
    );
    let e = copy(&source(b"payload"), &dst, &manifest(b"payload"), &c)
        .await
        .unwrap_err();
    assert_eq!(e.cause, Cause::Timeout);
    assert_eq!(e.publication, Publication::Unknown);
    assert_eq!(dst.inner.writes.load(Ordering::SeqCst), 1);
}
#[tokio::test(flavor = "current_thread")]
async fn acknowledged_publication_retains_certainty_on_read_failure_or_cancellation() {
    for cancellation in [false, true] {
        let token = CancellationToken::new();
        let c = Context::new(
            tokio::time::Instant::now() + Duration::from_secs(5),
            token.clone(),
        );
        let dst = Faults {
            fail_after_create: !cancellation,
            cancel_after_create: if cancellation { Some(token) } else { None },
            ..Faults::default()
        };
        let e = copy(&source(b"payload"), &dst, &manifest(b"payload"), &c)
            .await
            .unwrap_err();
        assert_eq!(e.publication, Publication::Confirmed);
        assert_eq!(e.progress.verified, 0);
        assert_eq!(
            e.cause,
            if cancellation {
                Cause::Canceled
            } else {
                Cause::Backend
            }
        );
        assert_eq!(
            dst.inner
                .get(manifest(b"payload").entries()[0].key(), 7)
                .await
                .unwrap(),
            b"payload"
        );
    }
}
#[tokio::test(flavor = "current_thread")]
async fn denied_destination_does_not_fall_back_to_source_or_create() {
    let dst = Faults {
        get_error: Some(rom_blob::Error::Denied),
        ..Faults::default()
    };
    let e = copy(&Store::default(), &dst, &manifest(b"payload"), &ctx())
        .await
        .unwrap_err();
    assert_eq!(e.cause, Cause::Denied);
    assert_eq!(dst.inner.writes.load(Ordering::SeqCst), 0);
}
#[tokio::test(flavor = "current_thread")]
async fn corrupt_source_missing_source_and_oversized_source_are_refused() {
    for b in [
        None,
        Some(b"CORRUPT".as_slice()),
        Some(b"too-long-payload".as_slice()),
    ] {
        let s = Store::default();
        if let Some(b) = b {
            s.objects
                .lock()
                .unwrap()
                .insert(Digest::of(b"payload").as_str().into(), b.into());
        }
        let dst = Store::default();
        let e = copy(&s, &dst, &manifest(b"payload"), &ctx())
            .await
            .unwrap_err();
        assert_eq!(
            e.cause,
            match b {
                None => Cause::Missing,
                Some(b) if b.len() > 7 => Cause::Limit,
                _ => Cause::Integrity,
            }
        );
        assert_eq!(dst.writes.load(Ordering::SeqCst), 0);
    }
}
#[tokio::test(flavor = "current_thread")]
async fn later_failure_reports_verified_prefix_and_never_cleans_up() {
    let first = manifest(b"payload").entries()[0].clone();
    let second = manifest(b"missing").entries()[0].clone();
    let m = Manifest::new(
        Digest::of(b"checkpoint"),
        vec![first, second],
        Limits::default(),
    )
    .unwrap();
    let dst = Store::default();
    let e = copy(&source(b"payload"), &dst, &m, &ctx())
        .await
        .unwrap_err();
    assert_eq!(e.index, 1);
    assert_eq!(e.progress.verified, 1);
    assert_eq!(e.progress.bytes, 7);
    assert_eq!(dst.get(m.entries()[0].key(), 7).await.unwrap(), b"payload");
}
#[test]
fn aggregate_object_entry_and_wire_limits_are_independent() {
    let entry = manifest(b"payload").entries()[0].clone();
    for l in [
        Limits {
            max_entries: 0,
            ..Limits::default()
        },
        Limits {
            max_object_bytes: 6,
            ..Limits::default()
        },
        Limits {
            max_total_bytes: 6,
            ..Limits::default()
        },
        Limits {
            max_json_bytes: 1,
            ..Limits::default()
        },
    ] {
        assert_eq!(
            Manifest::new(Digest::of(b"checkpoint"), vec![entry.clone()], l).unwrap_err(),
            Cause::Limit
        );
    }
    let bytes = manifest(b"payload").encode().unwrap();
    let mut w: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    w["extra"] = serde_json::json!("private-canary");
    assert_eq!(
        Manifest::decode(&serde_json::to_vec(&w).unwrap(), Limits::default()).unwrap_err(),
        Cause::Invalid
    );
    w.as_object_mut().unwrap().remove("extra");
    w["entries"][0]["key"] = serde_json::json!("../../private-canary");
    let err = Manifest::decode(&serde_json::to_vec(&w).unwrap(), Limits::default()).unwrap_err();
    assert!(!format!("{err:?} {err}").contains("private-canary"));
}

#[tokio::test(flavor = "current_thread")]
async fn opaque_rom_physical_key_is_not_the_content_digest() {
    let key = ObjectKey::parse(Digest::of(b"physical-publication-identity").as_str()).unwrap();
    let src = Store::default();
    src.objects
        .lock()
        .unwrap()
        .insert(key.as_str().into(), b"payload".into());
    let m = Manifest::new(
        Digest::of(b"checkpoint"),
        vec![Entry::new(key.clone(), Digest::of(b"payload"), 7)],
        Limits::default(),
    )
    .unwrap();
    let dst = Store::default();
    assert_eq!(copy(&src, &dst, &m, &ctx()).await.unwrap().verified, 1);
    assert_eq!(dst.get(&key, 7).await.unwrap(), b"payload");
}

#[tokio::test(flavor = "current_thread")]
async fn empty_and_zero_length_manifests_are_verified_under_explicit_deadline() {
    let empty = Manifest::new(Digest::of(b"checkpoint"), vec![], Limits::default()).unwrap();
    let dst = Store::default();
    assert_eq!(copy(&dst, &dst, &empty, &ctx()).await.unwrap().verified, 0);
    let expired = Context::new(
        tokio::time::Instant::now() - Duration::from_secs(1),
        CancellationToken::new(),
    );
    assert_eq!(
        verify(&dst, &empty, &expired).await.unwrap_err().cause,
        Cause::Timeout
    );
    let src = source(b"");
    let m = manifest(b"");
    assert_eq!(copy(&src, &dst, &m, &ctx()).await.unwrap().bytes, 0);
}
#[test]
fn aggregate_overflow_is_rejected() {
    let entries = vec![
        Entry::new(
            ObjectKey::parse(Digest::of(b"key1").as_str()).unwrap(),
            Digest::of(b"x"),
            u64::MAX,
        ),
        Entry::new(
            ObjectKey::parse(Digest::of(b"key2").as_str()).unwrap(),
            Digest::of(b"x"),
            1,
        ),
    ];
    assert_eq!(
        Manifest::new(
            Digest::of(b"checkpoint"),
            entries,
            Limits {
                max_object_bytes: usize::MAX,
                max_total_bytes: u64::MAX,
                ..Limits::default()
            }
        )
        .unwrap_err(),
        Cause::Limit
    );
}

#[tokio::test(flavor = "current_thread")]
async fn concurrent_create_conflict_requires_matching_readback() {
    for valid in [false, true] {
        let dst = Faults {
            concurrent_content: Some(if valid {
                b"payload".to_vec()
            } else {
                b"CORRUPT".to_vec()
            }),
            ..Faults::default()
        };
        let result = copy(&source(b"payload"), &dst, &manifest(b"payload"), &ctx()).await;
        if valid {
            let r = result.unwrap();
            assert_eq!(r.existing, 1);
            assert_eq!(r.created, 0);
        } else {
            let e = result.unwrap_err();
            assert_eq!(e.cause, Cause::Integrity);
            assert_eq!(e.publication, Publication::Conflict);
        }
        assert_eq!(dst.inner.writes.load(Ordering::SeqCst), 1);
    }
}
#[tokio::test(flavor = "current_thread")]
async fn cancellation_during_admitted_create_retains_unknown_publication() {
    let cancel = CancellationToken::new();
    let dst = Faults {
        stall_create: true,
        cancel_after_create: Some(cancel.clone()),
        ..Faults::default()
    };
    let c = Context::new(tokio::time::Instant::now() + Duration::from_secs(5), cancel);
    let e = copy(&source(b"payload"), &dst, &manifest(b"payload"), &c)
        .await
        .unwrap_err();
    assert_eq!(e.cause, Cause::Canceled);
    assert_eq!(e.publication, Publication::Unknown);
    assert_eq!(dst.inner.writes.load(Ordering::SeqCst), 1);
}
