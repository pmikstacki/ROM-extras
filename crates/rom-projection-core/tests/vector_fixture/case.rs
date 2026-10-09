//! Shared actual SQLite/redb source, with controlled candidates and isolated mutable authority.
use super::search_permissions::{Permissions, permission};
use rom::{Access, Actor, Command, Key, Resource, Runtime, Storage};
use rom_projection_core::{
    ApprovedVectorQuery, DocumentMapping, ProjectionProfile, SearchCandidate, SearchFailure,
    TargetFailure, VectorMetric, VectorQuery, VectorSearch, VectorSearchPolicy, VectorSearchScope,
    VectorSearchTarget,
};
use std::{
    future::Future,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};
#[derive(Clone, Resource)]
#[resource(name = "vector_core_docs")]
struct Document {
    input: String,
    other: String,
    readable: bool,
}
fn actor(subject: &str) -> Actor {
    Actor::trusted("vector-fixture", subject)
}
fn mapping() -> DocumentMapping {
    DocumentMapping::new(
        ProjectionProfile::new(
            "fixture",
            "controlled-vector",
            "mapping",
            Some("fixed-model"),
        )
        .unwrap(),
        vec!["input".into(), "other".into()],
        Some(3),
    )
    .unwrap()
}
struct Policy(Arc<AtomicBool>);
impl VectorSearchPolicy for Policy {
    async fn authorize(&mut self, actor: &Actor, scope: &VectorSearchScope) -> bool {
        assert_eq!(scope.kind(), Document::KIND);
        assert_eq!(scope.fields(), &["input".to_string(), "other".to_string()]);
        assert_eq!(scope.model_id(), "fixed-model");
        assert_eq!(scope.metric(), VectorMetric::Dot);
        assert_eq!(scope.dimensions(), 3);
        actor.subject.starts_with("reader-") && self.0.load(Ordering::SeqCst)
    }
}
struct Target {
    profile: ProjectionProfile,
    dimensions: usize,
    case: usize,
    calls: Arc<AtomicUsize>,
    candidates: Vec<SearchCandidate>,
    release: Option<tokio::sync::oneshot::Receiver<()>>,
}
impl VectorSearchTarget for Target {
    fn profile(&self) -> &ProjectionProfile {
        &self.profile
    }
    fn physical_target(&self) -> &str {
        "vector-generation"
    }
    fn dimensions(&self) -> usize {
        self.dimensions
    }
    fn metric(&self) -> VectorMetric {
        VectorMetric::Dot
    }
    async fn candidates(
        &mut self,
        query: &ApprovedVectorQuery,
    ) -> Result<Vec<SearchCandidate>, TargetFailure> {
        assert_eq!(format!("{query:?}"), "ApprovedVectorQuery");
        assert_eq!(query.profile(), &self.profile);
        assert_eq!(query.query().vector(), &[3., 4., 0.]);
        self.calls.fetch_add(1, Ordering::SeqCst);
        if let Some(release) = self.release.take() {
            release.await.unwrap();
        }
        if self.case == 11 {
            self.profile =
                ProjectionProfile::new("changed", "controlled", "mapping", Some("fixed-model"))
                    .unwrap();
        }
        if self.case == 15 {
            self.dimensions = 4;
        }
        if self.case == 12 {
            return Err(TargetFailure::Unknown);
        }
        Ok(std::mem::take(&mut self.candidates))
    }
}
fn candidate(id: &str, revision: u64) -> SearchCandidate {
    SearchCandidate::new(
        Key {
            kind: Document::KIND.into(),
            id: id.into(),
        },
        revision,
    )
    .unwrap()
}
static NEXT: AtomicUsize = AtomicUsize::new(0);
pub fn run(redb: bool, case: usize) {
    use std::os::unix::fs::DirBuilderExt;
    let unique = NEXT.fetch_add(1, Ordering::Relaxed);
    let directory =
        std::env::temp_dir().join(format!("rom-vector-core-{}-{unique}", std::process::id()));
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&directory)
        .unwrap();
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let row = Arc::new(AtomicBool::new(true));
    let field = Arc::new(AtomicBool::new(true));
    let reader = actor(&format!("reader-{unique}"));
    let _permission = Permissions::new(reader.clone(), row.clone(), field.clone());
    let storage: Arc<dyn Storage> = if redb {
        Arc::new(rom_redb::Redb::open(directory.join("resources")).unwrap())
    } else {
        Arc::new(rom_sqlite::Sqlite::open(directory.join("resources")).unwrap())
    };
    let runtime = Runtime::builder()
        .resource(
            Document::definition()
                .policy(|a, access, d| {
                    a.authority == "vector-fixture"
                        && (a.subject == "writer"
                            || (matches!(access, Access::Read)
                                && d.readable
                                && permission(a).is_some_and(|p| p.0.load(Ordering::SeqCst))))
                })
                .field_policy(|a, _, field, _| {
                    a.subject == "writer"
                        || field == "input"
                        || (field == "other"
                            && permission(a).is_some_and(|p| p.1.load(Ordering::SeqCst)))
                }),
        )
        .build(storage, Runtime::shared_cpu_pool(2).unwrap())
        .unwrap();
    rt.block_on(async {
        for id in ["a", "b", "c", "stale", "denied", "tomb"] {
            runtime
                .execute(
                    &actor("writer"),
                    Command::create(
                        id,
                        Document {
                            input: "current input".into(),
                            other: "current other".into(),
                            readable: id != "denied",
                        },
                    )
                    .idempotency(id),
                )
                .await
                .unwrap();
        }
        runtime
            .execute(
                &actor("writer"),
                Command::replace(
                    "stale",
                    Document {
                        input: "changed".into(),
                        other: "current other".into(),
                        readable: true,
                    },
                )
                .at_revision(1)
                .idempotency("replace-stale"),
            )
            .await
            .unwrap();
        runtime
            .execute(
                &actor("writer"),
                Command::<Document>::delete("tomb")
                    .at_revision(1)
                    .idempotency("delete-tomb"),
            )
            .await
            .unwrap();
    });
    let closed = [5, 6, 7, 9, 10].contains(&case);
    if closed {
        rt.block_on(runtime.shutdown()).unwrap();
    }
    let grant = Arc::new(AtomicBool::new(case != 0));
    let calls = Arc::new(AtomicUsize::new(0));
    let (send, receive) = tokio::sync::oneshot::channel();
    let candidates = match case {
        1 => vec![
            candidate("stale", 1),
            candidate("denied", 1),
            candidate("missing", 1),
            candidate("tomb", 2),
            candidate("a", 1),
            candidate("b", 1),
        ],
        5 => (0..9)
            .map(|n| candidate(&format!("excess-{n}"), 1))
            .collect(),
        6 => vec![
            SearchCandidate::new(
                Key {
                    kind: "foreign".into(),
                    id: "a".into(),
                },
                1,
            )
            .unwrap(),
        ],
        7 => vec![candidate("a", 1), candidate("a", 1)],
        8 => vec![candidate("c", 1), candidate("a", 1), candidate("b", 1)],
        16 => vec![],
        _ => vec![candidate("a", 1)],
    };
    let mapping = mapping();
    let target = Target {
        profile: mapping.profile().clone(),
        dimensions: 3,
        case,
        calls: calls.clone(),
        candidates,
        release: ([2, 3, 4, 12].contains(&case)).then_some(receive),
    };
    let mut search = VectorSearch::new(
        runtime.clone(),
        reader,
        Document::KIND,
        target,
        Policy(grant.clone()),
        mapping,
    )
    .unwrap();
    let mut query = VectorQuery::new(
        if case == 14 {
            vec![3., 4.]
        } else {
            vec![3., 4., 0.]
        },
        if case == 8 { 1 } else { 2 },
        8,
    )
    .unwrap();
    if [10, 13].contains(&case) {
        query = query
            .excluding(Key {
                kind: if case == 13 {
                    "foreign"
                } else {
                    Document::KIND
                }
                .into(),
                id: "a".into(),
            })
            .unwrap();
    }
    let mut pending = Box::pin(search.execute(query));
    if [2, 3, 4, 12].contains(&case) {
        rt.block_on(std::future::poll_fn(|cx| {
            assert!(pending.as_mut().poll(cx).is_pending());
            std::task::Poll::Ready(())
        }));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        match case {
            2 | 12 => grant.store(false, Ordering::SeqCst),
            3 => row.store(false, Ordering::SeqCst),
            4 => field.store(false, Ordering::SeqCst),
            _ => unreachable!(),
        }
        send.send(()).unwrap();
    }
    let result = rt.block_on(pending);
    match case {
        0 | 2 | 12 => assert!(matches!(result, Err(SearchFailure::Denied))),
        3 | 4 | 16 => assert!(result.unwrap().is_empty()),
        5 | 6 | 7 | 10 => assert!(matches!(
            result,
            Err(SearchFailure::Target(TargetFailure::Rejected))
        )),
        9 => assert!(matches!(result, Err(SearchFailure::SourceUnavailable))),
        11 | 15 => assert!(matches!(
            result,
            Err(SearchFailure::Core(rom_projection_core::Error::Conflict))
        )),
        13 | 14 => assert!(matches!(
            result,
            Err(SearchFailure::Core(rom_projection_core::Error::Invalid))
        )),
        _ => {
            let views = result.unwrap();
            assert_eq!(views.len(), if case == 1 { 2 } else { 1 });
            assert_eq!(views[0].key.id, if case == 8 { "c" } else { "a" });
            for view in views {
                let fields = view.value.unwrap();
                assert_eq!(fields["input"], "current input");
                assert_eq!(fields["other"], "current other");
                assert!(!fields.contains_key("readable"));
            }
        }
    }
    assert_eq!(
        calls.load(Ordering::SeqCst),
        usize::from(![0, 13, 14].contains(&case))
    );
    if !closed {
        rt.block_on(runtime.shutdown()).unwrap();
    }
    // Retain the actual SQLite/redb source and all test evidence.
}
