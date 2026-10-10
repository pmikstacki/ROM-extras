//! Catch false observations from changed native vector, payload or acknowledgement.
use super::reconcile::{completed, same_point};
use serde_json::json;
#[test]
fn native_completion_requires_success_and_completed_operation_identity() {
    assert!(
        completed(&json!({"status":"ok","result":{"operation_id":1,"status":"completed"}})).is_ok()
    );
    for v in [
        json!({"status":"ok","result":{"operation_id":1,"status":"acknowledged"}}),
        json!({"status":"error","result":{"operation_id":1,"status":"completed"}}),
        json!({"status":"ok","result":{"status":"completed"}}),
    ] {
        assert!(completed(&v).is_err());
    }
}
#[test]
fn native_points_require_exact_payload_and_float32_roundtrip() {
    let expected = json!({"id":"uuid","payload":{"rom_id":"original","rom_live":true},"vector":{"embedding":[3_f32,4_f32,f32::from_bits(1),f32::MAX]}});
    let mut actual = expected.clone();
    actual["vector"]["embedding"][2] = json!(f64::from(f32::from_bits(1)));
    assert!(same_point(&expected, &actual).is_ok());
    actual["vector"]["embedding"][0] = json!(0.6);
    assert!(same_point(&expected, &actual).is_err());
    actual = expected.clone();
    actual["payload"]["rom_id"] = json!("collision");
    assert!(same_point(&expected, &actual).is_err());
    actual = expected.clone();
    actual["vector"]["extra"] = json!([1.]);
    assert!(same_point(&expected, &actual).is_err());
    actual = expected.clone();
    actual["vector"]["embedding"][0] = json!(1e100);
    assert!(same_point(&expected, &actual).is_err());
}

#[test]
fn complete_candidates_never_admit_mixed_components_or_one_bit_drift() {
    use super::reconcile::same_point_candidates;
    let expected =
        json!({"id":"uuid","payload":{"rom_id":"ą /exact"},"vector":{"embedding":[0.6,0.8]}});
    let alternatives = vec![vec![0.6_f32, 0.8_f32], vec![0.5_f32, 0.9_f32]];
    let mut actual = expected.clone();
    for vector in &alternatives {
        actual["vector"]["embedding"] = json!(vector);
        assert!(same_point_candidates(&expected, &actual, Some(&alternatives)).is_ok());
    }
    actual["vector"]["embedding"] = json!([0.6_f32, 0.9_f32]);
    assert!(same_point_candidates(&expected, &actual, Some(&alternatives)).is_err());
    actual["vector"]["embedding"] = json!([f32::from_bits(0.6_f32.to_bits() + 1), 0.8_f32]);
    assert!(same_point_candidates(&expected, &actual, Some(&alternatives)).is_err());
    actual = expected.clone();
    actual["vector"]["extra"] = json!([1.]);
    assert!(same_point_candidates(&expected, &actual, Some(&alternatives)).is_err());
}
