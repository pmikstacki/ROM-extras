//! Numerical boundaries and complete-vector reconciliation; not native CPU qualification.
use crate::cosine::{Path, candidates, native_length, normalize};
use rom_projection_core::Error;
fn bits(v: &[f32]) -> Vec<u32> {
    v.iter().map(|x| x.to_bits()).collect()
}
#[test]
fn general_normalization_handles_extremes_zero_and_signed_zero() {
    assert_eq!(
        bits(&normalize(&[3., 4., -0.]).unwrap()),
        bits(&[0.6, 0.8, -0.])
    );
    assert_eq!(normalize(&[f32::MAX, 0., 0.]).unwrap(), vec![1., 0., 0.]);
    assert_eq!(
        bits(&normalize(&[f32::MAX, -f32::from_bits(1)]).unwrap()),
        bits(&[1., -0.])
    );
    assert_eq!(
        normalize(&[f32::from_bits(1), 0., 0.]).unwrap(),
        vec![1., 0., 0.]
    );
    for v in [
        vec![],
        vec![0., -0.],
        vec![f32::NAN],
        vec![f32::INFINITY],
        vec![1.; 4097],
    ] {
        assert!(matches!(normalize(&v), Err(Error::Invalid)));
    }
    for n in [1, 3, 15, 16, 17, 31, 32, 33, 4096] {
        let v = normalize(&vec![f32::MAX; n]).unwrap();
        assert!(v.iter().all(|x| x.is_finite() && *x > 0.));
        assert!((v.iter().map(|x| f64::from(*x).powi(2)).sum::<f64>() - 1.).abs() < 1e-6);
        assert!(!candidates(&v).is_empty());
    }
}
#[test]
fn eligible_candidates_follow_dimension_dispatch_and_preserve_bits() {
    for (n, count) in [(1, 1), (15, 1), (16, 3), (31, 3), (32, 4), (4096, 4)] {
        let mut v = vec![0.; n];
        v[0] = 1.;
        v[n - 1] = -0.;
        if n == 1 {
            v[0] = 1.;
        }
        let result = candidates(&v);
        assert_eq!(result.len(), count);
        assert!(result.iter().all(|x| bits(x) == bits(&v)));
    }
}
#[test]
fn tagged_reductions_and_tails_are_distinct_and_nonzero() {
    let mut v = vec![0.0005; 4096];
    v[0] = 1.;
    let scalar = native_length(&v, Path::Scalar);
    let avx = native_length(&v, Path::Avx);
    assert_ne!(scalar.to_bits(), avx.to_bits());
    for n in [16, 17, 31, 32, 33] {
        let v = vec![0.5; n];
        for path in [Path::Scalar, Path::Sse, Path::Avx, Path::Neon] {
            assert_eq!(native_length(&v, path), n as f32 / 4.);
        }
    }
}

#[test]
fn tagged_cutoff_is_not_a_readback_tolerance() {
    let inside = f32::from_bits(1_f32.to_bits() + 4);
    let outside = f32::from_bits(1_f32.to_bits() + 5);
    assert_eq!(bits(&candidates(&[inside])[0]), bits(&[inside]));
    assert_eq!(bits(&candidates(&[outside])[0]), bits(&[1.]));
    // A submitted host-normalized vector is still checked against full native outcomes.
    assert_ne!(bits(&[inside]), bits(&[outside]));
}
