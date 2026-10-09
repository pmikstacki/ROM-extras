//! Public finite input and non-forgeable approval boundaries.
use rom::Key;
use rom_projection_core::{Error, VectorQuery};
#[test]
fn vector_query_validates_shape_limits_and_exact_exclusion_without_disclosing_components() {
    for vector in [
        vec![],
        vec![f32::NAN],
        vec![f32::INFINITY],
        vec![f32::NEG_INFINITY],
    ] {
        assert!(matches!(
            VectorQuery::new(vector, 1, 1),
            Err(Error::Invalid)
        ));
    }
    assert!(matches!(
        VectorQuery::new(vec![0.; 4097], 1, 1),
        Err(Error::TooLarge)
    ));
    for (limit, budget) in [(0, 1), (65, 65), (2, 1), (1, 257)] {
        assert!(VectorQuery::new(vec![1., 0., 0.], limit, budget).is_err());
    }
    let query = VectorQuery::new(vec![-0., 3., 4.], 64, 256)
        .unwrap()
        .excluding(Key {
            kind: "documents".into(),
            id: "exact/Original-ID".into(),
        })
        .unwrap();
    assert_eq!(query.vector()[0].to_bits(), 0_f32.to_bits());
    assert_eq!(query.excluded().unwrap().id, "exact/Original-ID");
    assert_eq!(query.result_limit(), 64);
    assert_eq!(query.candidate_budget(), 256);
    assert_eq!(format!("{query:?}"), "VectorQuery");
}
