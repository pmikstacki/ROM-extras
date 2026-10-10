//! Public admission boundaries, independent of network availability.
use rom_projection_core::{Error, ProjectionProfile};
use rom_qdrant::{Distance, Generation};
fn profile() -> ProjectionProfile {
    ProjectionProfile::new("deployment", "qdrant", "mapping", Some("model")).unwrap()
}
#[test]
fn generation_admits_only_exact_float32_metrics_and_bounded_concrete_names() {
    for metric in [Distance::Dot, Distance::Euclid, Distance::Manhattan] {
        let g = Generation::new(profile(), "unique_generation", "host_nonce", 3, metric).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&g.definition().unwrap()).unwrap();
        assert_eq!(value["vectors"]["embedding"]["size"], 3);
        assert_eq!(value["vectors"]["embedding"]["datatype"], "float32");
        assert_eq!(value["shard_number"], 1);
        assert_eq!(value["replication_factor"], 1);
    }
    assert!(matches!(
        Generation::new(profile(), "valid", "nonce", 3, Distance::Cosine),
        Err(Error::Unsupported)
    ));
    for name in ["", "alias/path", "..", "name?query", "with spaces"] {
        assert!(Generation::new(profile(), name, "nonce", 3, Distance::Dot).is_err());
    }
    for dimension in [0, 4097] {
        assert!(Generation::new(profile(), "valid", "nonce", dimension, Distance::Dot).is_err());
    }
    assert!(matches!(
        Generation::new(profile(), &"a".repeat(129), "nonce", 3, Distance::Dot),
        Err(Error::TooLarge)
    ));
    assert!(Generation::new(profile(), "valid", "", 3, Distance::Dot).is_err());
}

#[test]
fn cosine_profile_is_explicit_version_bound_and_preserves_old_refusal() {
    let g = Generation::cosine_v1_19_2(profile(), "cosine", "nonce", 4096).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&g.definition().unwrap()).unwrap();
    assert_eq!(value["vectors"]["embedding"]["distance"], "Cosine");
    assert_eq!(
        value["metadata"]["rom_extras_projection"]["normalization"],
        "cosine-f64-qdrant-1.19.2-v1"
    );
    for n in [0, 4097] {
        assert!(Generation::cosine_v1_19_2(profile(), "cosine", "nonce", n).is_err());
    }
    assert!(Generation::cosine_v1_19_2(profile(), "a/path", "nonce", 3).is_err());
}
