use rom::{Actor, Command, Key, Resource, Runtime, Storage};
use rom_map_core::{Cancellation, Error, RequestContext};
use rom_maps_host_example::{read_points, read_session_points};
use std::{sync::Arc, time::Duration};
#[derive(Clone, Resource)]
#[resource(name = "map_places")]
struct Place {
    title: String,
    longitude: rom::FiniteF64,
    latitude: rom::FiniteF64,
    private_note: String,
}
#[tokio::test]
async fn public_runtime_disclosure_sqlite_and_redb() {
    for redb in [false, true] {
        let path = std::env::temp_dir().join(format!(
            "rom-map-host-{}-{redb}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        let storage: Arc<dyn Storage> = if redb {
            Arc::new(rom_redb::Redb::open(path.join("db")).unwrap())
        } else {
            Arc::new(rom_sqlite::Sqlite::open(path.join("db")).unwrap())
        };
        let runtime = Runtime::builder()
            .resource(
                Place::definition()
                    .policy(|actor, _, _| {
                        actor.authority == "map-host-fixture" && actor.subject != "denied"
                    })
                    .field_policy(|actor, _, field, _| {
                        actor.subject == "writer"
                            || (field != "private_note"
                                && (actor.subject != "hidden" || field != "longitude"))
                    }),
            )
            .build(storage, Runtime::shared_cpu_pool(2).unwrap())
            .unwrap();
        let writer = Actor::trusted("map-host-fixture", "writer");
        let id = "ę/0001:Straße";
        for (id, longitude) in [(id, 21.0), ("invalid", 181.0), ("tomb", 21.0)] {
            runtime
                .execute(
                    &writer,
                    Command::create(
                        id,
                        Place {
                            title: "Approved place".into(),
                            longitude: rom::FiniteF64::new(longitude).unwrap(),
                            latitude: rom::FiniteF64::new(52.0).unwrap(),
                            private_note: "never send to UI".into(),
                        },
                    )
                    .idempotency(id),
                )
                .await
                .unwrap();
        }
        runtime
            .execute(
                &writer,
                Command::<Place>::delete("tomb")
                    .at_revision(1)
                    .idempotency("delete-tomb"),
            )
            .await
            .unwrap();
        let keys: Vec<_> = [id, "missing", "invalid", "tomb"]
            .into_iter()
            .map(|id| Key {
                kind: Place::KIND.into(),
                id: id.into(),
            })
            .collect();
        let ctx = RequestContext::new(Duration::from_secs(5), Cancellation::new()).unwrap();
        let reader = Actor::trusted("map-host-fixture", "reader");
        let points = read_points(&runtime, &reader, Place::KIND, &keys, &ctx)
            .await
            .unwrap();
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].location.key(), &keys[0]);
        assert_eq!(
            points[0].location.coordinate().longitude_latitude(),
            [21.0, 52.0]
        );
        if let Some(output) = std::env::var_os("ROM_MAP_FIXTURE_SNAPSHOT") {
            let payload = rom::json!({"kind": Place::KIND, "points": points.iter().map(|p| rom::json!({
                "id": p.location.key().id, "title": p.title,
                "longitude": p.location.coordinate().longitude_degrees(),
                "latitude": p.location.coordinate().latitude_degrees()
            })).collect::<Vec<_>>()});
            let encoded = payload.to_string();
            assert!(!encoded.contains("private_note"));
            assert!(!encoded.contains("never send to UI"));
            std::fs::write(output, encoded).unwrap();
        }
        let view = runtime
            .read_projected(&reader, Place::KIND, id)
            .await
            .unwrap();
        assert!(!view.value.unwrap().contains_key("private_note"));
        for subject in ["hidden", "denied"] {
            assert!(
                read_points(
                    &runtime,
                    &Actor::trusted("map-host-fixture", subject),
                    Place::KIND,
                    &keys,
                    &ctx
                )
                .await
                .unwrap()
                .is_empty()
            );
        }
        assert!(matches!(
            read_session_points(&runtime, &reader, Place::KIND, &keys, &ctx, || false).await,
            Err(Error::Rejected)
        ));
        let mut checks = 0;
        assert!(matches!(
            read_session_points(&runtime, &reader, Place::KIND, &keys, &ctx, || {
                checks += 1;
                checks == 1
            })
            .await,
            Err(Error::Rejected)
        ));
        assert_eq!(checks, 2);
        assert_eq!(
            read_session_points(&runtime, &reader, Place::KIND, &keys, &ctx, || true)
                .await
                .unwrap()
                .len(),
            1
        );
        let duplicate = [keys[0].clone(), keys[0].clone()];
        assert!(matches!(
            read_points(&runtime, &reader, Place::KIND, &duplicate, &ctx).await,
            Err(Error::InvalidQuery)
        ));
        assert!(matches!(
            read_points(&runtime, &reader, "Other", &keys, &ctx).await,
            Err(Error::InvalidQuery)
        ));
        let too_many = vec![keys[0].clone(); 201];
        assert!(matches!(
            read_points(&runtime, &reader, Place::KIND, &too_many, &ctx).await,
            Err(Error::TooLarge)
        ));
        let token = Cancellation::new();
        token.cancel();
        let cancelled = RequestContext::new(Duration::from_secs(5), token).unwrap();
        assert!(matches!(
            read_points(&runtime, &reader, Place::KIND, &keys, &cancelled).await,
            Err(Error::Cancelled)
        ));
    }
}
