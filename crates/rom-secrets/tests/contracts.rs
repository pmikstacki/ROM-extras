//! Host reference and secret-material rejection contracts.
use rom_secrets::{Error, ResolvedSecret, SecretBytes, SecretRef, Version};

#[test]
fn references_cannot_select_urls_paths_or_ambiguous_names() {
    for invalid in [
        "",
        "https://evil",
        "../key",
        "a/b",
        "a%2fb",
        "a?version=1",
        "a#b",
        "a b",
        "a\n",
        "é",
        "1key",
    ] {
        assert!(matches!(SecretRef::new(invalid), Err(Error::Invalid)));
    }
    assert!(SecretRef::new("a".repeat(65)).is_err());
    assert_eq!(
        SecretRef::new("oidc-client_2").unwrap().as_str(),
        "oidc-client_2"
    );
}

#[test]
fn positive_version_and_nonempty_bounded_credentials_are_required() {
    assert!(matches!(Version::pinned(0), Err(Error::Invalid)));
    assert_eq!(
        Version::pinned(7).unwrap(),
        Version::Pinned(std::num::NonZeroU64::new(7).unwrap())
    );
    assert!(matches!(SecretBytes::new(vec![0; 4097]), Err(Error::Limit)));
    let bytes = SecretBytes::new(Vec::new()).unwrap();
    assert!(matches!(ResolvedSecret::new(bytes, 1), Err(Error::Invalid)));
    assert!(matches!(
        ResolvedSecret::new(SecretBytes::new(vec![1]).unwrap(), 0),
        Err(Error::Invalid)
    ));
    let resolved = ResolvedSecret::new(SecretBytes::new(vec![0; 4096]).unwrap(), 8).unwrap();
    assert_eq!(resolved.version(), 8);
    assert_eq!(resolved.bytes().expose().len(), 4096);
}
