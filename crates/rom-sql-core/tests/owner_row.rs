//! Public ownership-row validation without a native SDK or database.
use rom_sql_core::{OwnerError, validate_owner_row};

#[test]
fn exact_native_fields_preserve_idle_and_active_states() {
    for identity in [[0; 32], [7; 32]] {
        for generation in [0, 1, i64::MAX] {
            let idle =
                validate_owner_row(1, &identity, Some(&identity), generation, None, None).unwrap();
            assert_eq!(idle.generation(), generation as u64);
            assert_eq!(idle.token(), None);
        }
        let token = [9; 32];
        let active =
            validate_owner_row(1, &identity, Some(&identity), 1, Some(&token), Some(32)).unwrap();
        assert_eq!(active.token().unwrap().as_bytes(), &token);
    }
}

#[test]
fn unsupported_format_is_distinct_from_malformed_fields() {
    for format in [i64::MIN, 0, 2, i64::MAX] {
        assert_eq!(
            validate_owner_row(format, &[0; 32], None, -1, None, Some(-1)),
            Err(OwnerError::UnsupportedFormat)
        );
    }
}

#[test]
fn immutable_identity_requires_all_exact_bytes() {
    let expected = [7; 32];
    for identity in [
        None,
        Some(&[7; 31][..]),
        Some(&[7; 33][..]),
        Some(&[8; 32][..]),
    ] {
        assert_eq!(
            validate_owner_row(1, &expected, identity, 1, None, None),
            Err(OwnerError::Invalid)
        );
    }
}

#[test]
fn token_projection_and_native_length_must_agree() {
    let identity = [7; 32];
    for (token, length) in [
        (Some(&[9; 32][..]), None),
        (None, Some(32)),
        (Some(&[9; 31][..]), Some(32)),
        (Some(&[9; 33][..]), Some(32)),
        (Some(&[0; 32][..]), Some(32)),
        (None, Some(31)),
        (None, Some(33)),
        (None, Some(-1)),
        (None, Some(i64::MAX)),
    ] {
        assert_eq!(
            validate_owner_row(1, &identity, Some(&identity), 1, token, length),
            Err(OwnerError::Invalid)
        );
    }
}

#[test]
fn signed_generation_rejects_negative_and_zero_active_state() {
    let identity = [7; 32];
    for generation in [i64::MIN, -1] {
        assert_eq!(
            validate_owner_row(1, &identity, Some(&identity), generation, None, None),
            Err(OwnerError::Invalid)
        );
    }
    assert_eq!(
        validate_owner_row(1, &identity, Some(&identity), 0, Some(&[9; 32]), Some(32)),
        Err(OwnerError::Invalid)
    );
}
