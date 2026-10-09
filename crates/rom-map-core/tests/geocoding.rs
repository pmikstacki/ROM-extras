//! Catch invalid queries, invented accuracy, truncated results and altered public ROM identity.
use rom_map_core::{
    Accuracy, Attribution, Coordinate, Error, GeocodeQuery, GeocodeResult, GeocodeResults,
    Provenance, ResourceLocation,
};
fn result() -> GeocodeResult {
    GeocodeResult::new(
        "Kraków",
        Coordinate::new(19.94, 50.06).unwrap(),
        Accuracy::Unknown,
        Provenance::new(
            "nominatim",
            "node:000123",
            Attribution::new(
                "© OpenStreetMap contributors",
                Some("https://www.openstreetmap.org/copyright"),
            )
            .unwrap(),
        )
        .unwrap(),
        None,
    )
    .unwrap()
}
#[test]
fn query_preserves_unicode_and_rejects_empty_overlong_and_unbounded_limits() {
    assert_eq!(
        GeocodeQuery::new("Kraków Rynek", 3).unwrap().text(),
        "Kraków Rynek"
    );
    for (text, limit) in [
        ("", 1),
        ("  ", 1),
        ("Kraków", 0),
        ("Kraków", 41),
        ("secret\nheader", 1),
    ] {
        assert!(matches!(
            GeocodeQuery::new(text, limit),
            Err(Error::InvalidQuery)
        ));
    }
    assert!(matches!(
        GeocodeQuery::new(&"x".repeat(1025), 1),
        Err(Error::TooLarge)
    ));
}
#[test]
fn empty_results_are_successful_and_over_limit_results_are_rejected() {
    assert_eq!(GeocodeResults::new(vec![], 1).unwrap().results().len(), 0);
    assert_eq!(
        GeocodeResults::new(vec![result()], 1)
            .unwrap()
            .results()
            .len(),
        1
    );
    assert!(matches!(
        GeocodeResults::new(vec![result(), result()], 1),
        Err(Error::TooLarge)
    ));
    assert!(matches!(
        GeocodeResults::new(vec![], 0),
        Err(Error::InvalidQuery)
    ));
}
#[test]
fn result_keeps_unknown_accuracy_and_exact_native_identity() {
    let result = result();
    assert!(matches!(result.accuracy(), Accuracy::Unknown));
    assert_eq!(result.provenance().source_id(), "node:000123");
    assert_eq!(
        result.provenance().attribution().link(),
        Some("https://www.openstreetmap.org/copyright")
    );
}
#[test]
fn attribution_cannot_embed_credentials_or_active_content() {
    for link in [
        "javascript:alert(1)",
        "https://user:secret@example.test/copyright",
    ] {
        assert!(matches!(
            Attribution::new("credit", Some(link)),
            Err(Error::InvalidResponse)
        ));
    }
    assert!(matches!(
        Attribution::new("", None),
        Err(Error::InvalidResponse)
    ));
    assert!(matches!(
        Attribution::new("<script>alert(1)</script>", None),
        Err(Error::InvalidResponse)
    ));
}
#[test]
fn resource_keys_remain_exact_without_case_folding_or_separator_reconstruction() {
    let key = rom::Key {
        kind: "Places".into(),
        id: "ę/0001:Straße".into(),
    };
    let location = ResourceLocation::new(key.clone(), Coordinate::new(19.0, 50.0).unwrap());
    assert_eq!(location.key(), &key);
}
