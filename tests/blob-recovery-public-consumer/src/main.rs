mod capture;
mod fixtures;
mod native;
use rom_blob::{BlobStore, Digest, ObjectKey};
use rom_blob_recovery::{Cause, Context, Entry, Limits, Manifest, Publication, copy, verify};
use std::{
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio_util::sync::CancellationToken;
pub(crate) fn ctx() -> Context {
    Context::new(
        tokio::time::Instant::now() + Duration::from_secs(30),
        CancellationToken::new(),
    )
}
pub(crate) fn unique(label: &str) -> Vec<u8> {
    format!(
        "recovery-{label}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    )
    .into_bytes()
}
pub(crate) fn manifest(b: &[u8]) -> Manifest {
    Manifest::new(
        Digest::of(b"host-approved-checkpoint"),
        vec![Entry::new(
            ObjectKey::parse(Digest::of(b).as_str()).unwrap(),
            Digest::of(b),
            b.len() as u64,
        )],
        Limits::default(),
    )
    .unwrap()
}
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let root = PathBuf::from(
        std::env::var("ROM_EXTRAS_BLOB_RECOVERY_RUN")
            .expect("private retained run directory required"),
    );
    assert!(root.is_dir());
    let azure = fixtures::emulator(1024);
    let s3 = fixtures::adapter(1024, false);
    for reverse in [false, true] {
        let bytes = unique(if reverse {
            "s3-to-azure"
        } else {
            "azure-to-s3"
        });
        let m = manifest(&bytes);
        let (src, dst): (&dyn BlobStore, &dyn BlobStore) = if reverse {
            (&s3, &azure)
        } else {
            (&azure, &s3)
        };
        src.create(m.entries()[0].key(), bytes.clone())
            .await
            .unwrap();
        let report = copy(src, dst, &m, &ctx()).await.unwrap();
        assert_eq!(report.created, 1);
        assert_eq!(report.bytes, bytes.len() as u64);
        assert_eq!(dst.get(m.entries()[0].key(), 1024).await.unwrap(), bytes);
        assert_eq!(copy(src, dst, &m, &ctx()).await.unwrap().existing, 1);
        assert_eq!(verify(dst, &m, &ctx()).await.unwrap().verified, 1);
    }
    println!(
        "actual S3/Azurite bidirectional create-only copy, independent reads and explicit equal resume passed"
    );
    let bytes = unique("native-lost-ack");
    let m = manifest(&bytes);
    azure
        .create(m.entries()[0].key(), bytes.clone())
        .await
        .unwrap();
    let proxy = fixtures::Proxy::start();
    let destination = fixtures::proxy_adapter(proxy.endpoint());
    let failure = copy(&azure, &destination, &m, &ctx()).await.unwrap_err();
    assert_eq!(failure.cause, Cause::Unknown);
    assert_eq!(failure.publication, Publication::Unknown);
    assert_eq!(failure.progress.verified, 0);
    let c = proxy.counts();
    assert_eq!((c.puts, c.successes, c.dropped), (1, 1, 1));
    assert_eq!(s3.get(m.entries()[0].key(), 1024).await.unwrap(), bytes);
    assert_eq!(
        copy(&azure, &destination, &m, &ctx())
            .await
            .unwrap()
            .existing,
        1
    );
    assert_eq!(proxy.counts().puts, 1);
    proxy.drain().await;
    println!(
        "actual successful S3 PUT response loss: Unknown, independent publication proof and explicit read-only resume passed"
    );
    native::run(&root).await;
    fixtures::restart().await;
    println!("same retained S3 fixture restart completed; verifying fresh clients");
    // Fresh clients prove retained object readability after native container restart.
    let fresh = fixtures::adapter(1024, false);
    // Fixture readiness observations retry reads only, outside the recovery operation.
    let ready_deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    loop {
        if fresh.get(m.entries()[0].key(), 1024).await.is_ok() {
            break;
        }
        assert!(
            tokio::time::Instant::now() < ready_deadline,
            "bounded retained fixture readiness failed"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert_eq!(verify(&fresh, &m, &ctx()).await.unwrap().verified, 1);
    println!(
        "blob recovery public consumer passed; retained native databases, archives and manifests"
    );
}
