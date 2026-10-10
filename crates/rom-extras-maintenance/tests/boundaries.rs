//! Public admission, capability refusal and safe uncertainty classification.
use rom_extras_maintenance::{Backend, Error, Limits, restore};
#[test]
fn uncertain_publication_and_private_converter_errors_remain_distinct() {
    assert_eq!(Error::from(rom::Error::Unknown), Error::Unknown);
    assert_eq!(
        Error::from(rom::Error::invalid(
            "private-value-canary",
            "private-key-canary"
        )),
        Error::Invalid
    );
    assert_eq!(
        Error::from(rom::Error::Unsupported("private-endpoint-canary".into())),
        Error::Unsupported
    );
    for error in [Error::Unknown, Error::Invalid, Error::Unsupported] {
        assert!(!format!("{error:?} {error}").contains("canary"));
    }
}
#[test]
fn empty_and_excessive_host_budgets_are_rejected() {
    assert_eq!(Limits::new(0, 1).unwrap_err(), Error::Invalid);
    assert_eq!(Limits::new(1, 0).unwrap_err(), Error::Invalid);
    assert_eq!(
        Limits::new(128 * 1024 * 1024 + 1, 1).unwrap_err(),
        Error::TooLarge
    );
    assert_eq!(Limits::new(1, 400001).unwrap_err(), Error::TooLarge);
}
#[test]
fn disabled_native_profiles_refuse_before_opening_paths() {
    let path = std::path::Path::new("/nonexistent-maintenance-capability-canary");
    // Native features must be checked before even an archive read could fail Unavailable.
    let mut backends = Vec::new();
    if !cfg!(feature = "sqlite") {
        backends.push(Backend::Sqlite);
    }
    if !cfg!(feature = "redb") {
        backends.push(Backend::Redb);
    }
    for backend in backends {
        assert!(matches!(
            restore(backend, path, path, Limits::default()),
            Err(Error::Unsupported)
        ));
    }
}
