//! Actual native dimensional/extreme representation and exact readback; public API only.
use crate::native::Host;
use reqwest::Method;
use rom::{JournalView, Key, ProjectedView, json};
use rom_projection_core::{ProjectionTarget, TargetFailure};
use rom_qdrant::PreparedWrite;
pub async fn run(host: Host) {
    for (i, generation) in host.dimensional_generations().iter().enumerate() {
        let definition: serde_json::Value =
            serde_json::from_slice(&generation.definition().unwrap()).unwrap();
        let dimensions = definition["vectors"]["embedding"]["size"].as_u64().unwrap() as usize;
        let mapping = Host::mapping_for_dimensions(dimensions);
        let mut target = host.dimensional_target(i);
        target.verify_generation().await.unwrap();
        let vectors = vec![
            vec![f32::MAX; dimensions],
            vec![f32::from_bits(1); dimensions],
            (0..dimensions)
                .map(|n| if n == 0 { f32::MAX } else { -f32::from_bits(1) })
                .collect(),
            (0..dimensions)
                .map(|n| if n % 2 == 0 { 3. } else { -4. })
                .collect(),
            (0..dimensions)
                .map(|n| if n == 0 { 1. } else { 1e-3 })
                .collect(),
            (0..dimensions)
                .map(|n| if n == 0 { 1. } else { 1e-4 })
                .collect(),
            (0..dimensions)
                .map(|n| if n == 0 { -3. } else { -0. })
                .collect(),
        ];
        for (case, vector) in vectors.into_iter().enumerate() {
            let document = mapping
                .document(
                    &JournalView {
                        position: 1,
                        view: ProjectedView {
                            key: Key {
                                kind: "documents".into(),
                                id: format!("ą / exact-dimension-{dimensions}-{case}"),
                            },
                            revision: u64::MAX,
                            value: Some(json!({"value":u64::MAX}).as_object().unwrap().clone()),
                        },
                    },
                    Some(vector.clone()),
                )
                .unwrap();
            let original = document.metadata().clone();
            let page = target.prepare(&[&document]).unwrap();
            let observations = target.apply(page).await.unwrap();
            let cursor = |position| rom::JournalCursor {
                generation: "native-cosine".into(),
                kind: "documents".into(),
                position,
            };
            let intent = rom_projection_core::PageIntent::new(
                &rom_projection_core::Checkpoint::new(target.physical_target(), vec![cursor(0)])
                    .unwrap(),
                cursor(1),
                vec![original.clone()],
            )
            .unwrap();
            intent.reconcile(mapping.profile(), observations).unwrap();
            assert_eq!(document.vector().unwrap(), vector);
            intent
                .reconcile(
                    mapping.profile(),
                    target
                        .inspect_prepared(&target.prepare(&[&document]).unwrap())
                        .await
                        .unwrap(),
                )
                .unwrap();
            target
                .apply(target.prepare(&[&document]).unwrap())
                .await
                .unwrap();
            if case == 0 {
                // Native upload of a different direction would normalize away a one-bit magnitude perturbation.
                // Mutate payload digest instead; complete native corruption is also covered by regular consumer.
                let value: serde_json::Value =
                    serde_json::from_slice(PreparedWrite::new(&document).unwrap().as_bytes())
                        .unwrap();
                let point = &value["points"][0];
                let mut payload = point["payload"].clone();
                payload["rom_digest"] = json!("changed");
                host.control(
                    Method::PUT,
                    &format!(
                        "collections/{}/points/payload?wait=true&ordering=strong",
                        target.physical_target()
                    ),
                    json!({"payload":payload,"points":[point["id"]]}),
                )
                .await;
                assert_eq!(
                    target
                        .inspect_prepared(&target.prepare(&[&document]).unwrap())
                        .await
                        .err(),
                    Some(TargetFailure::Rejected)
                );
            }
        }
    }
    println!("Native Cosine dimensional/extreme exact reconciliation passed");
}
