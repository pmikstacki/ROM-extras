//! KMS input and envelope boundary contracts.
use rom_kms::{Binding, Envelope, KeyRef};
use rom_secrets::{Error, SecretRef};
#[test]
fn context_and_aad_have_distinct_bounded_inputs() {
    assert!(matches!(
        Binding::new(Vec::new(), Vec::new()),
        Err(Error::Invalid)
    ));
    assert!(matches!(
        Binding::new(vec![0; 4097], Vec::new()),
        Err(Error::Limit)
    ));
    assert!(matches!(
        Binding::new(vec![1], vec![0; 4097]),
        Err(Error::Limit)
    ));
    let binding = Binding::new(vec![1], vec![2]).unwrap();
    assert_eq!(binding.context(), &[1]);
    assert_eq!(binding.aad(), &[2]);
}
#[test]
fn stored_envelopes_require_identity_version_and_bounded_ciphertext() {
    let profile = SecretRef::new("openbao-test").unwrap();
    let key = KeyRef::new("application-key").unwrap();
    assert!(matches!(
        Envelope::new(profile.clone(), key.clone(), 0, "ciphertext".into()),
        Err(Error::Invalid)
    ));
    assert!(matches!(
        Envelope::new(profile.clone(), key.clone(), 1, String::new()),
        Err(Error::Invalid)
    ));
    assert!(matches!(
        Envelope::new(profile.clone(), key.clone(), 1, "a".repeat(16385)),
        Err(Error::Limit)
    ));
    let envelope = Envelope::new(profile, key, 7, "vault:v7:ciphertext".into()).unwrap();
    assert_eq!(envelope.version(), 7);
    assert_eq!(envelope.key().as_str(), "application-key");
    assert_eq!(envelope.profile().as_str(), "openbao-test");
}
