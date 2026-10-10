//! Complete local preparation must precede durable intent.
use rom::{JournalView, Key, ProjectedView, json};
use rom_projection_core::{DocumentMapping, Error, ProjectionProfile, ProjectionTarget};
use rom_qdrant::{Distance, Generation, Qdrant, TlsConfig};
use std::time::Duration;
fn mapping() -> DocumentMapping {
    DocumentMapping::new(
        ProjectionProfile::new("deployment", "qdrant", "mapping", Some("model")).unwrap(),
        vec!["value".into()],
        Some(3),
    )
    .unwrap()
}
fn target(mapping: &DocumentMapping, dimensions: usize) -> Qdrant {
    let ca = include_bytes!("support/admission-ca.pem").to_vec();
    Qdrant::new(
        TlsConfig::api_key(
            "https://127.0.0.1:1",
            ca,
            b"explicit-host-token".to_vec(),
            Duration::from_secs(1),
        )
        .unwrap(),
        Generation::new(
            mapping.profile().clone(),
            "unique_generation",
            "nonce",
            dimensions,
            Distance::Dot,
        )
        .unwrap(),
    )
    .unwrap()
}
#[test]
fn page_rejects_duplicates_dimensions_profiles_and_count_before_network() {
    let m = mapping();
    let d = m
        .document(
            &JournalView {
                position: 1,
                view: ProjectedView {
                    key: Key {
                        kind: "documents".into(),
                        id: "exact/identifier".into(),
                    },
                    revision: u64::MAX,
                    value: Some(json!({"value":u64::MAX}).as_object().unwrap().clone()),
                },
            },
            Some(vec![3., 4., 0.]),
        )
        .unwrap();
    let t = target(&m, 3);
    assert!(t.prepare(&[&d]).is_ok());
    assert!(t.prepare(&[]).is_ok());
    assert!(matches!(t.prepare(&[&d, &d]), Err(Error::Conflict)));
    assert!(matches!(t.prepare(&vec![&d; 65]), Err(Error::TooLarge)));
    assert!(matches!(target(&m, 2).prepare(&[&d]), Err(Error::Invalid)));
    let wrong = DocumentMapping::new(
        ProjectionProfile::new("other", "qdrant", "mapping", Some("model")).unwrap(),
        vec!["value".into()],
        Some(3),
    )
    .unwrap();
    assert!(matches!(
        target(&wrong, 3).prepare(&[&d]),
        Err(Error::Conflict)
    ));
}

#[test]
fn page_bounds_total_wire_bytes_before_intent() {
    let mapping = mapping();
    let documents: Vec<_> = (0..64)
        .map(|i| {
            mapping
                .document(
                    &JournalView {
                        position: i + 1,
                        view: ProjectedView {
                            key: Key {
                                kind: "documents".into(),
                                id: format!("{i}"),
                            },
                            revision: 1,
                            value: Some(
                                json!({"value":"\n".repeat(16000)})
                                    .as_object()
                                    .unwrap()
                                    .clone(),
                            ),
                        },
                    },
                    Some(vec![3., 4., 0.]),
                )
                .unwrap()
        })
        .collect();
    let refs: Vec<_> = documents.iter().collect();
    assert!(matches!(
        target(&mapping, 3).prepare(&refs),
        Err(Error::TooLarge)
    ));
}

#[test]
fn cosine_preparation_rejects_zero_and_keeps_original_approved_vector() {
    let mapping = mapping();
    let target = Qdrant::new(
        TlsConfig::api_key(
            "https://127.0.0.1:1",
            include_bytes!("support/admission-ca.pem").to_vec(),
            b"host-token".to_vec(),
            Duration::from_secs(1),
        )
        .unwrap(),
        Generation::cosine_v1_19_2(mapping.profile().clone(), "cosine", "nonce", 3).unwrap(),
    )
    .unwrap();
    for vector in [
        vec![3., 4., -0.],
        vec![f32::MAX, 0., 0.],
        vec![f32::from_bits(1), 0., 0.],
        vec![0., -0., 0.],
    ] {
        let document = mapping
            .document(
                &JournalView {
                    position: 1,
                    view: ProjectedView {
                        key: Key {
                            kind: "documents".into(),
                            id: "ą / exact".into(),
                        },
                        revision: u64::MAX,
                        value: Some(json!({"value":u64::MAX}).as_object().unwrap().clone()),
                    },
                },
                Some(vector.clone()),
            )
            .unwrap();
        let metadata = document.metadata().clone();
        let approved_bits = document
            .vector()
            .unwrap()
            .iter()
            .map(|x| x.to_bits())
            .collect::<Vec<_>>();
        if vector.iter().all(|x| *x == 0.) {
            assert!(matches!(target.prepare(&[&document]), Err(Error::Invalid)));
        } else {
            assert!(target.prepare(&[&document]).is_ok());
        }
        assert!(document.metadata() == &metadata);
        assert_eq!(
            document
                .vector()
                .unwrap()
                .iter()
                .map(|x| x.to_bits())
                .collect::<Vec<_>>(),
            approved_bits
        );
    }
}
