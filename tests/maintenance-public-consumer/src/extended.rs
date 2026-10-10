//! Trusted native persistence fixtures; application actions remain in model/native scenarios.
use crate::model::{Before, plan};
use rom::*;
use rom_extras_maintenance::{
    Backend, BackupSource, Error as MaintenanceError, Limits, backup, preview_migration, restore,
};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{
        Arc, Barrier,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};
fn row() -> Row {
    Row {
        key: Key {
            kind: Before::KIND.into(),
            id: "protected.exact[0]".into(),
        },
        revision: 1,
        value: Some(
            Before {
                amount: "42".into(),
                enabled: false,
            }
            .encode(),
        ),
        protected: ProtectedMetadata {
            deletion_authorization: None,
            source_provenance: Some(SourceProvenance {
                source: "maint-source".into(),
                version: "source-v1".into(),
                generation: 7,
                field_origins: BTreeMap::from([
                    ("amount".into(), "private-origin".into()),
                    ("enabled".into(), "private-origin".into()),
                ]),
            }),
        },
    }
}
fn bundle(row: Row, id: &str, expected: Option<u64>, work: Vec<PendingWork>) -> Bundle {
    Bundle {
        expected,
        receipt: Receipt {
            retry_epoch: 0,
            identity: id.into(),
            fingerprint: format!("fingerprint-{id}"),
            row,
            replay_version: None,
        },
        changed: true,
        effects: vec![],
        reaction_limits: (!work.is_empty()).then(ReactionLimits::default),
        reactions: work,
        completed_work: None,
    }
}
pub fn run(root: &Path) {
    qualify(
        root,
        Backend::Sqlite,
        Arc::new(rom_sqlite::Sqlite::open(root.join("sqlite-work.db")).unwrap()),
    );
    qualify(
        root,
        Backend::Redb,
        Arc::new(rom_redb::Redb::open(root.join("redb-work.db")).unwrap()),
    );
}
fn qualify<S: Storage + BackupSource>(root: &Path, backend: Backend, source: Arc<S>) {
    let label = if backend == Backend::Sqlite {
        "sqlite"
    } else {
        "redb"
    };
    source
        .register(&[Before::descriptor().canonical().unwrap()])
        .unwrap();
    let original = row();
    let pending = PendingWork {
        id: "private-work".into(),
        cause: Cause {
            retry_epoch: 0,
            root: "private-work-create".into(),
            parent: None,
            depth: 1,
            started_at: 1,
            path: vec!["private-definition".into()],
        },
        definition: "private-definition".into(),
        version: 1,
        service_key: "private-service".into(),
        delivery_profile: DeliveryProfile::AtLeastOnce,
        payload: WorkPayload::Source(original.clone()),
    };
    source
        .commit(&bundle(
            original.clone(),
            "private-work-create",
            None,
            vec![pending],
        ))
        .unwrap();
    let claimed = match source
        .reaction_update(WorkUpdate::Claim { now: 1 })
        .unwrap()
    {
        WorkResult::Claimed(c) => c,
        _ => panic!("actual native Work claim required"),
    };
    let archive = root.join(format!("{label}-work.rombk"));
    let report = backup(source.as_ref(), &archive, Limits::default()).unwrap();
    assert_eq!(report.counts.work, 1);
    assert_eq!(
        preview_migration(backend, &archive, &plan(), Limits::default()),
        Err(MaintenanceError::Unsupported)
    );
    let validated = plan().validate_work(|w| {
        assert_eq!(w.definition, "private-definition");
        Ok(())
    });
    assert_eq!(
        preview_migration(backend, &archive, &validated, Limits::default())
            .unwrap()
            .changed_schemas,
        1
    );
    let dest = root.join(format!("{label}-work-restored.db"));
    let restored = restore(backend, &archive, &dest, Limits::default()).unwrap();
    assert_eq!(restored.load(&original.key).unwrap().unwrap(), original);
    assert_eq!(
        restored
            .receipt("private-work-create")
            .unwrap()
            .unwrap()
            .row
            .protected,
        original.protected
    );
    assert!(matches!(
        restored.reaction_update(WorkUpdate::Finish {
            claim: claimed.key(),
            now: 2,
            outcome: WorkOutcome::Done
        }),
        Err(Error::Conflict)
    ));
    let records = restored.reaction_records().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].attempts, claimed.work.attempts);
    assert!(records[0].generation > claimed.work.generation);
    drop(restored);
    // Native writes and coherent backup overlap in a live handle; every observer validates full snapshot consistency.
    let barrier = Arc::new(Barrier::new(2));
    let stop = Arc::new(AtomicBool::new(false));
    let revision = Arc::new(AtomicU64::new(1));
    let writer = source.clone();
    let start = barrier.clone();
    let writer_stop = stop.clone();
    let progress = revision.clone();
    let thread = std::thread::spawn(move || {
        start.wait();
        for next_revision in 2..=4096 {
            if writer_stop.load(Ordering::SeqCst) {
                break;
            }
            let mut next = row();
            next.revision = next_revision;
            next.value = Some(
                Before {
                    amount: next_revision.to_string(),
                    enabled: false,
                }
                .encode(),
            );
            writer
                .commit(&bundle(
                    next,
                    &format!("private-live-{next_revision}"),
                    Some(next_revision - 1),
                    vec![],
                ))
                .unwrap();
            progress.store(next_revision, Ordering::SeqCst);
            std::thread::yield_now();
        }
        // Finishing before the backup observation is an explicit failure, not a passing race.
        assert!(
            writer_stop.load(Ordering::SeqCst),
            "bounded writer exhausted before backup windows completed"
        );
    });
    barrier.wait();
    let mut witnessed = false;
    let mut windows = Vec::new();
    for index in 0..16 {
        let path = root.join(format!("{label}-live-{index}.rombk"));
        let before = revision.load(Ordering::SeqCst);
        backup(source.as_ref(), &path, Limits::default()).unwrap();
        let after = revision.load(Ordering::SeqCst);
        let (_, snapshot) =
            rom_backup::read(&path, backend, rom_backup::BackupLimits::default()).unwrap();
        snapshot.validate().unwrap();
        assert_eq!(snapshot.rows[0].revision as usize, snapshot.receipts.len());
        assert_eq!(snapshot.receipts.len(), snapshot.events.len());
        assert_eq!(snapshot.state.retry_epochs().current, 0);
        assert!(snapshot.rows[0].revision >= before && snapshot.rows[0].revision <= after);
        windows.push((before, snapshot.rows[0].revision, after));
        if after > before {
            witnessed = true;
            break;
        }
        std::thread::yield_now();
    }
    stop.store(true, Ordering::SeqCst);
    thread.join().unwrap();
    assert!(
        witnessed,
        "no acknowledged native write progress during any bounded backup invocation window"
    );
    assert_eq!(
        source.load(&row().key).unwrap().unwrap().revision,
        revision.load(Ordering::SeqCst)
    );
    println!(
        "{label}: acknowledged native revision progress (before, coherent snapshot, after): {windows:?}"
    );
    println!(
        "{label}: native protected provenance/receipts, claimed Work restore fencing, explicit pending-work migration validator and live backup coherence passed"
    );
}
