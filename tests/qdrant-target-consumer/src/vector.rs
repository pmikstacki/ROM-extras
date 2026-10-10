//! Real native ranking and current public ROM hydration; no fabricated query results.
use crate::{
    native::Host,
    search_permissions::{Permissions, permission},
};
use reqwest::Method;
use rom::{Access, Actor, Command, Key, Resource, Runtime, Storage};
use rom_projection_core::{
    HostVectors, ProjectionHistory, ProjectionTarget, RuntimeHistory, SearchFailure, TargetFailure,
    VectorInput, VectorMetric, VectorQuery, VectorSearch, VectorSearchPolicy, VectorSearchScope,
};
use serde_json::json;
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
#[derive(Clone, Resource)]
#[resource(name = "native_qdrant_query_docs")]
struct Document {
    value: u64,
    note: String,
    readable: bool,
}
struct Vectors {
    cosine: bool,
}
impl HostVectors for Vectors {
    fn model_id(&self) -> &str {
        "host-model-v1"
    }
    fn vector(&mut self, input: VectorInput<'_>) -> rom_projection_core::Result<Vec<f32>> {
        let value = input
            .fields()
            .iter()
            .find(|(name, _)| *name == "value")
            .unwrap()
            .1
            .as_u64()
            .unwrap();
        Ok(match value {
            1 => vec![1., 0., 0.],
            2 if self.cosine => vec![0., 3., 0.],
            2 => vec![3., 0., 0.],
            3 => vec![2., 3., 0.],
            _ => vec![value as f32, 0., 0.],
        })
    }
}
struct Policy {
    granted: Arc<AtomicBool>,
    metric: VectorMetric,
}
impl VectorSearchPolicy for Policy {
    async fn authorize(&mut self, actor: &Actor, scope: &VectorSearchScope) -> bool {
        assert_eq!(actor.authority, "native-vector");
        assert_eq!(scope.kind(), Document::KIND);
        assert_eq!(scope.fields(), &["value".to_string()]);
        assert_eq!(scope.model_id(), "host-model-v1");
        assert_eq!(scope.dimensions(), 3);
        assert_eq!(scope.metric(), self.metric);
        assert_eq!(format!("{scope:?}"), "VectorSearchScope");
        self.granted.load(Ordering::SeqCst)
    }
}
fn query() -> VectorQuery {
    VectorQuery::new(vec![1., 0., 0.], 3, 64).unwrap()
}
fn actor(subject: &str) -> Actor {
    Actor::trusted("native-vector", subject)
}
pub async fn run(host: Host, path: &Path, redb: bool) {
    let backend = if redb { "redb" } else { "sqlite" };
    let source_path = path.join("source");
    let storage: Arc<dyn Storage> = tokio::task::spawn_blocking(move || {
        if redb {
            Arc::new(rom_redb::Redb::open(source_path).unwrap()) as Arc<dyn Storage>
        } else {
            Arc::new(rom_sqlite::Sqlite::open(source_path).unwrap()) as Arc<dyn Storage>
        }
    })
    .await
    .unwrap();
    let row = Arc::new(AtomicBool::new(true));
    let field = Arc::new(AtomicBool::new(true));
    let reader = actor(&format!("reader-{backend}"));
    let _permission = Permissions::new(reader.clone(), row.clone(), field.clone());
    let runtime = Runtime::builder()
        .resource(
            Document::definition()
                .policy(|a, access, d| {
                    a.authority == "native-vector"
                        && (a.subject == "writer"
                            || (matches!(access, Access::Read)
                                && d.readable
                                && permission(a).is_some_and(|p| p.0.load(Ordering::SeqCst))))
                })
                .field_policy(|a, _, name, _| {
                    a.subject == "writer"
                        || name == "note"
                        || (name == "value"
                            && permission(a).is_some_and(|p| p.1.load(Ordering::SeqCst)))
                }),
        )
        .build(storage, Runtime::shared_cpu_pool(2).unwrap())
        .unwrap();
    let origin = runtime
        .journal_head(&actor("writer"), Document::KIND)
        .await
        .unwrap();
    let key = |suffix: &str| format!("{backend}-ą / {suffix}");
    for (i, suffix) in ["a", "b", "c", "stale", "denied", "tomb"]
        .iter()
        .enumerate()
    {
        runtime
            .execute(
                &actor("writer"),
                Command::create(
                    &key(suffix),
                    Document {
                        value: i as u64 + 1,
                        note: "current public value".into(),
                        readable: *suffix != "denied",
                    },
                )
                .idempotency(suffix),
            )
            .await
            .unwrap();
    }
    let mut history = RuntimeHistory::with_vectors(
        runtime.clone(),
        actor("writer"),
        Document::KIND,
        host.mapping(),
        Vectors {
            cosine: host.is_cosine(),
        },
    )
    .unwrap();
    let batch = history.fetch(&origin).await.unwrap();
    let docs: Vec<_> = batch
        .events
        .iter()
        .map(|e| history.document(e).unwrap())
        .collect();
    if !redb {
        let mut empty = VectorSearch::new(
            runtime.clone(),
            reader.clone(),
            Document::KIND,
            host.reader(0),
            Policy {
                granted: Arc::new(AtomicBool::new(true)),
                metric: if host.is_cosine() {
                    VectorMetric::Cosine
                } else {
                    VectorMetric::Dot
                },
            },
            host.mapping(),
        )
        .unwrap();
        assert!(empty.execute(query()).await.unwrap().is_empty());
    }
    for metric in 0..3 {
        let mut target = host.target(metric);
        target
            .apply(target.prepare(&docs.iter().collect::<Vec<_>>()).unwrap())
            .await
            .unwrap();
    }
    // Native filter qualification uses administrator-seeded foreign protocol points, never host-approved Resources.
    for metric in 0..3 {
        let body: serde_json::Value =
            serde_json::from_slice(rom_qdrant::PreparedWrite::new(&docs[0]).unwrap().as_bytes())
                .unwrap();
        let mut points = Vec::new();
        for (prefix, field, value) in [
            ("11111111", "rom_live", json!(false)),
            ("22222222", "rom_kind", json!("foreign-native-kind")),
            ("33333333", "rom_profile", json!("foreign-native-profile")),
        ] {
            let mut point = body["points"][0].clone();
            let id = point["id"].as_str().unwrap();
            point["id"] = json!(format!("{prefix}{}", &id[8..]));
            point["payload"][field] = value;
            point["vector"]["embedding"] = json!([100., 0., 0.]);
            points.push(point);
        }
        host.admin_call(
            Method::PUT,
            &format!(
                "collections/{}/points?wait=true&ordering=strong",
                host.target(metric).physical_target()
            ),
            json!({"points":points}),
        )
        .await;
    }
    runtime
        .execute(
            &actor("writer"),
            Command::replace(
                &key("stale"),
                Document {
                    value: 40,
                    note: "new revision".into(),
                    readable: true,
                },
            )
            .at_revision(1)
            .idempotency("stale-update"),
        )
        .await
        .unwrap();
    runtime
        .execute(
            &actor("writer"),
            Command::<Document>::delete(&key("tomb"))
                .at_revision(1)
                .idempotency("tomb-delete"),
        )
        .await
        .unwrap();
    for metric in 0..3 {
        let granted = Arc::new(AtomicBool::new(true));
        let mut search = VectorSearch::new(
            runtime.clone(),
            reader.clone(),
            Document::KIND,
            host.reader(metric),
            Policy {
                granted: granted.clone(),
                metric: if host.is_cosine() {
                    VectorMetric::Cosine
                } else {
                    [
                        VectorMetric::Dot,
                        VectorMetric::Euclid,
                        VectorMetric::Manhattan,
                    ][metric]
                },
            },
            host.mapping(),
        )
        .unwrap();
        let expected = if host.is_cosine() {
            vec![key("a"), key("c"), key("b")]
        } else if metric == 0 {
            vec![key("b"), key("c"), key("a")]
        } else {
            vec![key("a"), key("b"), key("c")]
        };
        let views = search.execute(query()).await.unwrap();
        assert_eq!(
            views.iter().map(|v| v.key.id.clone()).collect::<Vec<_>>(),
            expected
        );
        assert!(views.iter().all(
            |v| v.revision == 1 && v.value.as_ref().unwrap()["note"] == "current public value"
        ));
        // Exclusion is applied by the actual native query, preserving exact Unicode/slash keys.
        let views = search
            .execute(
                query()
                    .excluding(Key {
                        kind: Document::KIND.into(),
                        id: key("b"),
                    })
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(views.len(), 2);
        assert!(views.iter().all(|v| v.key.id != key("b")));
        let views = search
            .execute(VectorQuery::new(vec![1., 0., 0.], 1, 64).unwrap())
            .await
            .unwrap();
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].key.id, expected[0]);
        if host.is_cosine() {
            let before =
                host.control(Method::GET, "fixture/status", json!({})).await["native_requests"]
                    .as_u64()
                    .unwrap();
            assert!(matches!(
                search
                    .execute(VectorQuery::new(vec![0., -0., 0.], 3, 64).unwrap())
                    .await,
                Err(SearchFailure::Target(TargetFailure::Rejected))
            ));
            assert_eq!(
                host.control(Method::GET, "fixture/status", json!({})).await["native_requests"]
                    .as_u64(),
                Some(before)
            );
        }
        // Denial before dispatch must leave the actual relay query count unchanged.
        let before = host.control(Method::GET, "fixture/status", json!({})).await["queries"]
            .as_u64()
            .unwrap();
        granted.store(false, Ordering::SeqCst);
        assert!(matches!(
            search.execute(query()).await,
            Err(SearchFailure::Denied)
        ));
        assert_eq!(
            host.control(Method::GET, "fixture/status", json!({})).await["queries"].as_u64(),
            Some(before)
        );
        granted.store(true, Ordering::SeqCst);
        for mode in ["policy", "row", "field", "generation"] {
            host.control(Method::POST, "fixture/config", json!({"mode":"hold"}))
                .await;
            let mutate = async {
                let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
                loop {
                    if host.control(Method::GET, "fixture/status", json!({})).await["started"]
                        == true
                    {
                        break;
                    }
                    assert!(
                        tokio::time::Instant::now() < deadline,
                        "Native query never reached the held response"
                    );
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                match mode {
                    "policy" => granted.store(false, Ordering::SeqCst),
                    "row" => row.store(false, Ordering::SeqCst),
                    "field" => field.store(false, Ordering::SeqCst),
                    "generation" => {
                        host.admin_call(
                            Method::PATCH,
                            &format!("collections/{}", host.target(metric).physical_target()),
                            json!({"metadata":{"rom_extras_projection":{"generation":"drift"}}}),
                        )
                        .await;
                    }
                    _ => unreachable!(),
                }
                host.control(Method::POST, "fixture/release", json!({}))
                    .await;
            };
            let (result, ()) = tokio::join!(search.execute(query()), mutate);
            match mode {
                "policy" => assert!(matches!(result, Err(SearchFailure::Denied))),
                "generation" => assert!(matches!(
                    result,
                    Err(SearchFailure::Target(TargetFailure::Rejected))
                )),
                _ => assert!(result.unwrap().is_empty()),
            }
            granted.store(true, Ordering::SeqCst);
            row.store(true, Ordering::SeqCst);
            field.store(true, Ordering::SeqCst);
            if mode == "generation" {
                host.admin_call(
                    Method::PATCH,
                    &format!("collections/{}", host.target(metric).physical_target()),
                    json!({"metadata":host.definitions()[metric]["metadata"]}),
                )
                .await;
            }
            host.control(Method::POST, "fixture/config", json!({"mode":null}))
                .await;
        }
        if metric == 0 {
            for mode in [
                "429",
                "oversized",
                "redirect",
                "delay",
                "malformed",
                "cancel",
            ] {
                host.control(
                    Method::POST,
                    "fixture/config",
                    json!({"mode":if mode=="cancel" {"delay"} else {mode}}),
                )
                .await;
                let before =
                    host.control(Method::GET, "fixture/status", json!({})).await["queries"]
                        .as_u64()
                        .unwrap();
                if mode == "cancel" {
                    assert!(
                        tokio::time::timeout(Duration::from_millis(150), search.execute(query()))
                            .await
                            .is_err()
                    );
                } else {
                    assert!(matches!(
                        search.execute(query()).await,
                        Err(SearchFailure::Target(TargetFailure::Unknown))
                    ));
                }
                assert_eq!(
                    host.control(Method::GET, "fixture/status", json!({})).await["queries"]
                        .as_u64(),
                    Some(before + 1)
                );
                host.control(Method::POST, "fixture/config", json!({"mode":null}))
                    .await;
            }
        }
        if redb {
            // Actual native payload corruption must not become a rounded or native-version ROM revision.
            let body: serde_json::Value = serde_json::from_slice(
                rom_qdrant::PreparedWrite::new(&docs[1]).unwrap().as_bytes(),
            )
            .unwrap();
            let mut point = body["points"][0].clone();
            let path = format!(
                "collections/{}/points?wait=true&ordering=strong",
                host.target(metric).physical_target()
            );
            point["payload"]["rom_revision_hi"] = json!(u32::MAX as u64 + 1);
            host.admin_call(Method::PUT, &path, json!({"points":[point.clone()]}))
                .await;
            assert!(matches!(
                search.execute(query()).await,
                Err(SearchFailure::Target(TargetFailure::Rejected))
            ));
            point["payload"]["rom_revision_hi"] = json!(0);
            point["payload"]["rom_id"] = json!("corrupt-original-key");
            host.admin_call(Method::PUT, &path, json!({"points":[point]}))
                .await;
            assert!(matches!(
                search.execute(query()).await,
                Err(SearchFailure::Target(TargetFailure::Rejected))
            ));
        }
    }
    runtime.shutdown().await.unwrap();
    println!(
        "Native vector {backend}: three metrics, exact keys/revisions, current hydration, held-response policy/row/field/generation changes and separately authored refusals passed"
    );
}
