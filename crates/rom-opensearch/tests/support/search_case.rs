//! Real OpenSearch full-text query over public SQLite/redb journal documents.
#[path = "search_proxy.rs"]
mod search_proxy;
use super::search_permissions::{Permissions, permission};
use crate::native_fixture;
use rom::{Actor, Command, Resource, Runtime, Storage};
use rom_projection_core::{
    ProjectionHistory, ProjectionTarget, RuntimeHistory, Search, SearchFailure, SearchPolicy,
    SearchScope, TextMode, TextQuery,
};
use std::{
    os::unix::fs::DirBuilderExt,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
#[derive(Clone, Resource)]
#[resource(name = "native_search_docs")]
struct Doc {
    title: String,
    other: String,
    note: String,
}
struct Policy(Arc<AtomicBool>);
impl SearchPolicy for Policy {
    async fn authorize(&mut self, actor: &Actor, scope: &SearchScope) -> bool {
        actor.authority == "native-search-fixture"
            && scope.kind() == Doc::KIND
            && scope.field() == "title"
            && self.0.load(Ordering::SeqCst)
    }
}
pub fn run(redb: bool) {
    run_case(redb, 0);
}
/// A denied protected-field query dispatches no requests to the actual relay.
pub fn run_denied(redb: bool) {
    run_case(redb, 4);
}
/// Revoke query (1), row (2) or field (3) authority while an actual native reply is held.
pub fn run_revocation(redb: bool, case: usize) {
    assert!((1..=3).contains(&case));
    run_case(redb, case);
}
fn run_case(redb: bool, case: usize) {
    let directory = std::env::temp_dir().join(format!(
        "rom-opensearch-search-{}-{redb}-{case}",
        std::process::id()
    ));
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&directory)
        .unwrap();
    let storage: Arc<dyn Storage> = if redb {
        Arc::new(rom_redb::Redb::open(directory.join("resources")).unwrap())
    } else {
        Arc::new(rom_sqlite::Sqlite::open(directory.join("resources")).unwrap())
    };
    let query_grant = Arc::new(AtomicBool::new(true));
    let row_grant = Arc::new(AtomicBool::new(true));
    let field_grant = Arc::new(AtomicBool::new(true));
    let reader = Actor::trusted(
        "native-search-fixture",
        &format!("native-reader-{}-{redb}-{case}", std::process::id()),
    );
    let _permissions = Permissions::new(reader.clone(), row_grant.clone(), field_grant.clone());
    let runtime = Runtime::builder()
        .resource(
            Doc::definition()
                .policy(|actor, _, _| {
                    actor.authority == "native-search-fixture"
                        && (actor.subject == "writer"
                            || permission(actor).is_some_and(|p| p.0.load(Ordering::SeqCst)))
                })
                .field_policy(|actor, _, field, _| {
                    actor.subject == "writer"
                        || field == "note"
                        || (field == "title"
                            && permission(actor).is_some_and(|p| p.1.load(Ordering::SeqCst)))
                }),
        )
        .build(storage, Runtime::shared_cpu_pool(2).unwrap())
        .unwrap();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        let writer = Actor::trusted("native-search-fixture", "writer");
        let origin = runtime.journal_head(&writer, Doc::KIND).await.unwrap();
        for (id, title) in [
            ("a/雪", "wind rise"),
            ("b", "wind"),
            ("stale", "wind rise"),
            ("tomb", "wind rise"),
        ] {
            runtime
                .execute(
                    &writer,
                    Command::create(
                        id,
                        Doc {
                            title: title.into(),
                            other: "private source".into(),
                            note: "permitted note".into(),
                        },
                    )
                    .idempotency(id),
                )
                .await
                .unwrap();
        }
        let (mut target, mapping) = native_fixture::fixture();
        let profile = mapping.profile().clone();
        native_fixture::create_generation(&mut target).await;
        let mut history =
            RuntimeHistory::new(runtime.clone(), writer.clone(), Doc::KIND, mapping).unwrap();
        let page = history.fetch(&origin).await.unwrap();
        let documents: Vec<_> = page
            .events
            .iter()
            .map(|event| history.document(event).unwrap())
            .collect();
        let request = target
            .prepare(&documents.iter().collect::<Vec<_>>())
            .unwrap();
        target.apply(request).await.unwrap();
        let physical = target.physical_target().to_owned();
        let mut search = Search::new(
            runtime.clone(),
            reader.clone(),
            Doc::KIND,
            target,
            Policy(query_grant.clone()),
        )
        .unwrap();
        // Native write acknowledgement is distinct from eventual search refresh visibility.
        let refreshed = tokio::time::timeout(Duration::from_secs(45), async {
            loop {
                let result = search
                    .execute(
                        TextQuery::new("title", "wind rise", TextMode::AnyTerms, 8, 8).unwrap(),
                    )
                    .await
                    .unwrap();
                if result.len() == 4 {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await;
        assert!(
            refreshed.is_ok(),
            "native search refresh did not become visible"
        );
        if case == 4 {
            let proxy = search_proxy::SearchProxy::start();
            let target = native_fixture::target_at(
                &proxy.endpoint,
                profile.clone(),
                &physical,
                vec!["title".into()],
            );
            let mut denied = Search::new(
                runtime.clone(),
                reader.clone(),
                Doc::KIND,
                target,
                Policy(query_grant.clone()),
            )
            .unwrap();
            let result = denied
                .execute(
                    TextQuery::new("other", "private source", TextMode::AnyTerms, 8, 8).unwrap(),
                )
                .await;
            assert!(matches!(result, Err(SearchFailure::Denied)));
            assert_eq!(proxy.request_count(), 0);
            runtime.shutdown().await.unwrap();
            return;
        }
        if case != 0 {
            let proxy = search_proxy::SearchProxy::start();
            let held_target = native_fixture::target_at(
                &proxy.endpoint,
                profile.clone(),
                &physical,
                vec!["title".into()],
            );
            let mut held_search = Search::new(
                runtime.clone(),
                reader.clone(),
                Doc::KIND,
                held_target,
                Policy(query_grant.clone()),
            )
            .unwrap();
            let query = TextQuery::new("title", "wind rise", TextMode::AnyTerms, 8, 8).unwrap();
            let pending = held_search.execute(query);
            let revoke = async {
                proxy.held().await;
                match case {
                    1 => query_grant.store(false, Ordering::SeqCst),
                    2 => row_grant.store(false, Ordering::SeqCst),
                    3 => field_grant.store(false, Ordering::SeqCst),
                    _ => unreachable!(),
                }
                proxy.release();
            };
            let (result, ()) = tokio::join!(pending, revoke);
            if case == 1 {
                assert!(matches!(result, Err(SearchFailure::Denied)));
            } else {
                assert!(result.unwrap().is_empty());
            }
            if case == 3 {
                let value = runtime
                    .read_projected(&reader, Doc::KIND, "a/雪")
                    .await
                    .unwrap()
                    .value
                    .unwrap();
                assert_eq!(value["note"], "permitted note");
                assert!(!value.contains_key("title"));
                assert!(!value.contains_key("other"));
            }
            // Revocation changed authority alone, not native Resource revisions.
            for id in ["a/雪", "b", "stale", "tomb"] {
                assert_eq!(
                    runtime
                        .read_projected(&writer, Doc::KIND, id)
                        .await
                        .unwrap()
                        .revision,
                    1
                );
            }
            runtime.shutdown().await.unwrap();
            return;
        }
        // A real native response must honor the smaller finite candidate budget.
        assert_eq!(
            search
                .execute(TextQuery::new("title", "wind rise", TextMode::AnyTerms, 2, 2).unwrap())
                .await
                .unwrap()
                .len(),
            2
        );
        runtime
            .execute(
                &writer,
                Command::replace(
                    "stale",
                    Doc {
                        title: "current changed".into(),
                        other: "private source".into(),
                        note: "permitted note".into(),
                    },
                )
                .at_revision(1)
                .idempotency("replace-stale"),
            )
            .await
            .unwrap();
        runtime
            .execute(
                &writer,
                Command::<Doc>::delete("tomb")
                    .at_revision(1)
                    .idempotency("delete-tomb"),
            )
            .await
            .unwrap();
        for (mode, expected) in [
            (TextMode::AllTerms, vec!["a/雪"]),
            (TextMode::AnyTerms, vec!["a/雪", "b"]),
        ] {
            let result = search
                .execute(TextQuery::new("title", "wind rise", mode, 8, 8).unwrap())
                .await
                .unwrap();
            let mut keys: Vec<_> = result.iter().map(|view| view.key.id.as_str()).collect();
            keys.sort();
            assert_eq!(keys, expected);
            for view in result {
                assert_eq!(view.revision, 1);
                assert!(!view.value.unwrap().contains_key("other"));
            }
        }
        runtime.shutdown().await.unwrap();
    });
    drop(runtime);
    std::fs::remove_dir_all(directory).unwrap();
}
