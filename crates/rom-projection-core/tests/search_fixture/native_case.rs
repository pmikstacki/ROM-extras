//! Shared actual public-storage hydration; candidate transport is controlled, not OpenSearch.
use super::search_permissions::{Permissions, permission};
use rom::{Access, Actor, Command, Resource, Runtime, Storage};
use rom_projection_core::{
    ApprovedTextQuery, ProjectionProfile, Search, SearchCandidate, SearchFailure, SearchPolicy,
    SearchScope, SearchTarget, TargetFailure, TextMode, TextQuery,
};
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};
#[derive(Clone, Resource)]
#[resource(name = "search_docs")]
struct Doc {
    title: String,
    note: String,
    secret: String,
    readable: bool,
}
fn actor(subject: &str) -> Actor {
    Actor::trusted("search-fixture", subject)
}
fn doc(readable: bool) -> Doc {
    Doc {
        title: "public current title".into(),
        note: "permitted note".into(),
        secret: "private source secret".into(),
        readable,
    }
}
fn runtime(redb: bool, path: &Path) -> Runtime {
    let storage: Arc<dyn Storage> = if redb {
        Arc::new(rom_redb::Redb::open(path).unwrap())
    } else {
        Arc::new(rom_sqlite::Sqlite::open(path).unwrap())
    };
    Runtime::builder()
        .resource(
            Doc::definition()
                .policy(|a, access, d| {
                    a.authority == "search-fixture"
                        && (a.subject == "writer"
                            || (matches!(access, Access::Read)
                                && d.readable
                                && permission(a).is_some_and(|p| p.0.load(Ordering::SeqCst))))
                })
                .field_policy(|a, _, f, _| {
                    a.subject == "writer"
                        || f == "note"
                        || (f == "title"
                            && permission(a).is_some_and(|p| p.1.load(Ordering::SeqCst)))
                }),
        )
        .build(storage, Runtime::shared_cpu_pool(2).unwrap())
        .unwrap()
}
struct Policy(Arc<AtomicBool>);
impl SearchPolicy for Policy {
    async fn authorize(&mut self, actor: &Actor, scope: &SearchScope) -> bool {
        actor.subject.starts_with("reader-")
            && scope.kind() == Doc::KIND
            && scope.field() == "title"
            && self.0.load(Ordering::SeqCst)
    }
}
struct Target {
    profile: ProjectionProfile,
    calls: Arc<AtomicUsize>,
    candidates: Vec<SearchCandidate>,
    release: Option<tokio::sync::oneshot::Receiver<()>>,
}
impl SearchTarget for Target {
    fn profile(&self) -> &ProjectionProfile {
        &self.profile
    }
    fn physical_target(&self) -> &str {
        "search-generation"
    }
    async fn candidates(
        &mut self,
        query: &ApprovedTextQuery,
    ) -> Result<Vec<SearchCandidate>, TargetFailure> {
        assert_eq!(query.scope().kind(), Doc::KIND);
        assert_eq!(query.profile(), &self.profile);
        assert_eq!(query.physical_target(), "search-generation");
        assert_eq!(format!("{query:?}"), "ApprovedTextQuery");
        self.calls.fetch_add(1, Ordering::SeqCst);
        if let Some(release) = self.release.take() {
            release.await.unwrap();
        }
        Ok(std::mem::take(&mut self.candidates))
    }
}
fn candidate(id: &str, revision: u64) -> SearchCandidate {
    SearchCandidate::new(
        rom::Key {
            kind: Doc::KIND.into(),
            id: id.into(),
        },
        revision,
    )
    .unwrap()
}
pub fn run(redb: bool, case: usize) {
    use std::future::Future;
    let directory = Directory::new();
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let row = Arc::new(AtomicBool::new(true));
    let field = Arc::new(AtomicBool::new(true));
    let reader = actor(&format!(
        "reader-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _permissions = Permissions::new(reader.clone(), row.clone(), field.clone());
    let native = runtime(redb, &directory.0.join("resources"));
    rt.block_on(async {
        for id in ["a", "b", "c", "stale", "denied", "tomb"] {
            native
                .execute(
                    &actor("writer"),
                    Command::create(id, doc(id != "denied")).idempotency(id),
                )
                .await
                .unwrap();
        }
        let mut changed = doc(true);
        changed.title = "changed current title".into();
        native
            .execute(
                &actor("writer"),
                Command::replace("stale", changed)
                    .at_revision(1)
                    .idempotency("replace-stale"),
            )
            .await
            .unwrap();
        native
            .execute(
                &actor("writer"),
                Command::<Doc>::delete("tomb")
                    .at_revision(1)
                    .idempotency("delete-tomb"),
            )
            .await
            .unwrap();
    });
    if (5..=7).contains(&case) || case == 9 {
        rt.block_on(native.shutdown()).unwrap();
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
        ],
        5 => (0..9)
            .map(|n| candidate(&format!("overrun-{n}"), 1))
            .collect(),
        6 => vec![
            SearchCandidate::new(
                rom::Key {
                    kind: "foreign".into(),
                    id: "a".into(),
                },
                1,
            )
            .unwrap(),
        ],
        7 => vec![candidate("a", 1), candidate("a", 1)],
        8 => vec![candidate("a", 1), candidate("b", 1), candidate("c", 1)],
        _ => vec![candidate("a", 1)],
    };
    let target = Target {
        profile: ProjectionProfile::new("fixture", "controlled", "mapping", None).unwrap(),
        calls: calls.clone(),
        candidates,
        release: (2..=4).contains(&case).then_some(receive),
    };
    let mut search = Search::new(
        native.clone(),
        reader,
        Doc::KIND,
        target,
        Policy(grant.clone()),
    )
    .unwrap();
    let query = TextQuery::new(
        if case == 10 { "secret" } else { "title" },
        "private query text",
        TextMode::AllTerms,
        2,
        8,
    )
    .unwrap();
    let mut pending = Box::pin(search.execute(query));
    if (2..=4).contains(&case) {
        rt.block_on(std::future::poll_fn(|cx| {
            assert!(pending.as_mut().poll(cx).is_pending());
            std::task::Poll::Ready(())
        }));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        match case {
            2 => grant.store(false, Ordering::SeqCst),
            3 => row.store(false, Ordering::SeqCst),
            4 => field.store(false, Ordering::SeqCst),
            _ => unreachable!(),
        }
        send.send(()).unwrap();
    }
    let result = rt.block_on(pending);
    match case {
        0 | 2 | 10 => assert!(matches!(result, Err(SearchFailure::Denied))),
        3 | 4 => assert!(result.unwrap().is_empty()),
        9 => assert!(matches!(result, Err(SearchFailure::SourceUnavailable))),
        5..=7 => assert!(matches!(
            result,
            Err(SearchFailure::Target(TargetFailure::Rejected))
        )),
        _ => {
            let views = result.unwrap();
            assert_eq!(views.len(), if case == 8 { 2 } else { 1 });
            for view in views {
                assert_eq!(view.revision, 1);
                let value = view.value.unwrap();
                assert_eq!(value["title"], "public current title");
                assert!(!value.contains_key("secret"));
                assert!(!value.contains_key("readable"));
            }
        }
    }
    assert_eq!(
        calls.load(Ordering::SeqCst),
        usize::from(case != 0 && case != 10)
    );
    if !(5..=7).contains(&case) && case != 9 {
        rt.block_on(native.shutdown()).unwrap();
    }
}
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Directory(std::path::PathBuf);
impl Directory {
    fn new() -> Self {
        use std::os::unix::fs::DirBuilderExt;
        let path = std::env::temp_dir().join(format!(
            "rom-extras-search-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&path)
            .unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
