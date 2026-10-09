//! Native Nginx reads and actual process recovery; this is host qualification code.
use crate::{context, model, source};
use rom::*;
use rom_import_transport::{Error as AcquisitionError, HttpConfig, HttpSource, StrongEtag};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
    sync::Arc,
    time::Duration,
};
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct HostRecord {
    invocation: Invocation,
    provenance: SourceProvenance,
    control: Key,
    control_revision: u64,
    valid_until: u64,
    authority: String,
    subject: String,
}
fn record(prepared: &rom_import::PreparedAction) -> HostRecord {
    let (invocation, _) = prepared.request();
    HostRecord {
        invocation,
        provenance: SourceProvenance {
            source: "deployment".into(),
            version: format!("sha256:{:x}", Sha256::digest(b"21")),
            generation: 2,
            field_origins: model::origins(),
        },
        control: Key {
            kind: model::Control::KIND.into(),
            id: "source".into(),
        },
        control_revision: 1,
        valid_until: u64::MAX,
        authority: "host".into(),
        subject: "worker".into(),
    }
}
fn save(path: &Path, record: &HostRecord) {
    let bytes = serde_json::to_vec(record).unwrap();
    assert!(bytes.len() <= 8192);
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).unwrap();
    file.write_all(&bytes).unwrap();
    file.sync_all().unwrap();
    File::open(path.parent().unwrap())
        .unwrap()
        .sync_all()
        .unwrap();
}
fn load(path: &Path) -> HostRecord {
    assert!(
        !std::fs::symlink_metadata(path)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    let file = File::open(path).unwrap();
    let metadata = file.metadata().unwrap();
    assert!(metadata.is_file());
    assert!(metadata.len() <= 8192);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
    }
    let mut bytes = Vec::new();
    file.take(8193).read_to_end(&mut bytes).unwrap();
    assert!(bytes.len() <= 8192);
    let record: HostRecord = serde_json::from_value(rom::parse_json(&bytes).unwrap()).unwrap();
    assert_eq!(record.authority, "host");
    assert_eq!(record.subject, "worker");
    assert_eq!(record.invocation.kind, model::Item::KIND);
    assert_eq!(record.invocation.id, model::ID);
    assert_eq!(record.provenance.source, "deployment");
    assert_eq!(record.control_revision, 1);
    record
}
fn permit(record: &HostRecord) -> SourcePermit {
    SourcePermit::trusted(
        Key {
            kind: record.invocation.kind.clone(),
            id: record.invocation.id.clone(),
        },
        record.provenance.clone(),
        RevisionCondition {
            key: record.control.clone(),
            revision: record.control_revision,
        },
        record.valid_until,
    )
    .unwrap()
}
fn actor(record: &HostRecord) -> Actor {
    Actor::trusted(&record.authority, &record.subject).with_kind(PrincipalKind::Service)
}
enum Backend {
    Sqlite(Arc<rom_sqlite::Sqlite>),
    Redb(Arc<rom_redb::Redb>),
}
impl Backend {
    fn open(path: &Path, redb: bool) -> Self {
        if redb {
            Self::Redb(Arc::new(rom_redb::Redb::open(path).unwrap()))
        } else {
            Self::Sqlite(Arc::new(rom_sqlite::Sqlite::open(path).unwrap()))
        }
    }
    fn runtime(&self) -> Runtime {
        match self {
            Self::Sqlite(s) => model::runtime_for(s.clone()),
            Self::Redb(s) => model::runtime_for(s.clone()),
        }
    }
    fn counts(&self) -> [u64; 4] {
        match self {
            Self::Sqlite(s) => s.counts().unwrap(),
            Self::Redb(s) => s.counts().unwrap(),
        }
    }
    fn fault(&self, after: bool) {
        match self {
            Self::Sqlite(s) => s.inject_fault(if after { 5 } else { 4 }),
            Self::Redb(s) => s.on_commit(Some(Arc::new(move |point| {
                if point == if after { usize::MAX } else { 0 } {
                    Err(Error::Unknown)
                } else {
                    Ok(())
                }
            }))),
        }
    }
}
async fn native_reads(endpoint: &str, ca: &[u8], token: &str, etag: &str) {
    let tag = StrongEtag::new(etag).unwrap();
    let (_sender, ctx) = context(2000);
    let p = crate::plan(b"21");
    let reader = source(endpoint, ca, token, Duration::ZERO);
    reader.fetch(&p, Some(&tag), &ctx).await.unwrap();
    let wrong = StrongEtag::new("\"incorrect\"").unwrap();
    assert_eq!(
        reader.fetch(&p, Some(&wrong), &ctx).await.unwrap_err(),
        AcquisitionError::PreconditionFailed
    );
    assert_eq!(
        reader
            .fetch(&crate::plan(b"22"), Some(&tag), &ctx)
            .await
            .unwrap_err(),
        AcquisitionError::Admission(rom_import::Error::DigestMismatch)
    );
    let denied = source(endpoint, ca, "WRONG", Duration::ZERO);
    assert_eq!(
        denied.fetch(&p, Some(&tag), &ctx).await.unwrap_err(),
        AcquisitionError::Rejected
    );
    let missing = source(
        &endpoint.replace("/input.json", "/missing.json"),
        ca,
        token,
        Duration::ZERO,
    );
    assert_eq!(
        missing.fetch(&p, None, &ctx).await.unwrap_err(),
        AcquisitionError::Missing
    );
    let no_auth = HttpSource::new(
        HttpConfig::new(endpoint, "host", Duration::ZERO, 1)
            .unwrap()
            .with_ca(ca.to_vec())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        no_auth.fetch(&p, None, &ctx).await.unwrap_err(),
        AcquisitionError::Rejected
    );
    let no_trust =
        HttpSource::new(HttpConfig::new(endpoint, "host", Duration::ZERO, 1).unwrap()).unwrap();
    assert_eq!(
        no_trust.fetch(&p, None, &ctx).await.unwrap_err(),
        AcquisitionError::Unavailable
    );
    println!(
        "native Nginx: static JSON, strong validator, changed digest, denied/missing auth, missing source and TLS refusal passed"
    );
}
async fn prepare(endpoint: &str, ca: &[u8], token: &str, etag: &str, root: &Path, redb: bool) {
    let name = if redb { "redb" } else { "sqlite" };
    let backend = Backend::open(&root.join(format!("{name}.db")), redb);
    let runtime = backend.runtime();
    model::seed(&runtime).await;
    assert_eq!(backend.counts(), [2, 2, 2, 0]);
    let (_sender, ctx) = context(2000);
    let tag = StrongEtag::new(etag).unwrap();
    let prepared = source(endpoint, ca, token, Duration::ZERO)
        .fetch(
            &model::plan(b"21", 1, model::origins(), "actual-unknown"),
            Some(&tag),
            &ctx,
        )
        .await
        .unwrap();
    save(
        &root.join(format!("host-request-{name}.json")),
        &record(&prepared),
    );
    runtime.shutdown().await.unwrap();
}
async fn attempt(root: &Path, redb: bool, after: bool) {
    let name = if redb { "redb" } else { "sqlite" };
    let record = load(&root.join(format!("host-request-{name}.json")));
    let backend = Backend::open(&root.join(format!("{name}.db")), redb);
    let runtime = backend.runtime();
    backend.fault(after);
    let result = runtime
        .invoke_sourced(&actor(&record), record.invocation.clone(), permit(&record))
        .await;
    if after {
        assert!(matches!(result, Err(Error::Unknown)));
        std::process::exit(71);
    } else {
        assert!(matches!(result, Err(Error::NotCommitted)));
        std::process::exit(70);
    }
}
async fn resume(root: &Path, redb: bool) {
    let name = if redb { "redb" } else { "sqlite" };
    let record = load(&root.join(format!("host-request-{name}.json")));
    let backend = Backend::open(&root.join(format!("{name}.db")), redb);
    let runtime = backend.runtime();
    let worker = actor(&record);
    assert_eq!(backend.counts(), [2, 3, 3, 0]);
    for _ in 0..2 {
        assert_eq!(
            runtime
                .invoke_sourced(&worker, record.invocation.clone(), permit(&record))
                .await
                .unwrap()
                .revision,
            2
        );
        assert_eq!(backend.counts(), [2, 3, 3, 0]);
    }
    let row = runtime
        .read::<model::Item>(&worker, model::ID)
        .await
        .unwrap();
    assert_eq!(row.revision, 2);
    let value = row.value.unwrap();
    assert_eq!(value.computed, 42);
    assert!(value.retained);
    assert_eq!(
        runtime
            .source_provenance(&worker, model::Item::KIND, model::ID)
            .await
            .unwrap(),
        Some(record.provenance.clone())
    );
    runtime
        .execute(
            &worker,
            Command::replace("source", model::Control { generation: 3 })
                .at_revision(1)
                .idempotency("new-generation"),
        )
        .await
        .unwrap();
    assert!(matches!(
        runtime
            .invoke_sourced(&worker, record.invocation.clone(), permit(&record))
            .await,
        Err(Error::Conflict)
    ));
    assert_eq!(
        runtime
            .read::<model::Item>(&worker, model::ID)
            .await
            .unwrap()
            .value
            .unwrap()
            .computed,
        42
    );
    runtime.shutdown().await.unwrap();
    println!(
        "{name}: actual precommit rollback, committed Unknown, child exit/reopen, exact receipt replay, native counts and original attribution passed; no refetch"
    );
}
pub fn dispatch(args: &[String]) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .enable_io()
        .build()
        .unwrap();
    match args[1].as_str() {
        "--native-read" => {
            assert_eq!(args.len(), 6);
            let ca = std::fs::read(&args[3]).unwrap();
            let config: serde_json::Value =
                rom::parse_json(&std::fs::read(&args[4]).unwrap()).unwrap();
            rt.block_on(native_reads(
                &args[2],
                &ca,
                config["token"].as_str().unwrap(),
                &args[5],
            ));
        }
        "--native-prepare" => {
            assert_eq!(args.len(), 7);
            let ca = std::fs::read(&args[3]).unwrap();
            let config: serde_json::Value =
                rom::parse_json(&std::fs::read(&args[4]).unwrap()).unwrap();
            for redb in [false, true] {
                rt.block_on(prepare(
                    &args[2],
                    &ca,
                    config["token"].as_str().unwrap(),
                    &args[5],
                    Path::new(&args[6]),
                    redb,
                ));
            }
        }
        "--native-attempt" => {
            assert_eq!(args.len(), 5);
            rt.block_on(attempt(
                Path::new(&args[2]),
                args[3] == "redb",
                args[4] == "after",
            ));
        }
        "--native-resume" => {
            assert_eq!(args.len(), 4);
            rt.block_on(resume(Path::new(&args[2]), args[3] == "redb"));
        }
        _ => panic!("unknown host fixture mode"),
    }
}
