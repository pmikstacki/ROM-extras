use crate::transit::encrypted;
use rom_kms::{EncryptionVersion, KeyRef};
use rom_secrets::{Error, SecretRef};
use serde_json::json;
#[test]
fn response_version_must_match_prefix_and_requested_version() {
    let profile = SecretRef::new("profile").unwrap();
    let key = KeyRef::new("key").unwrap();
    for data in [
        json!({"ciphertext":"vault:v2:opaque","key_version":3}),
        json!({"ciphertext":"vault:v2:opaque"}),
        json!({"ciphertext":"vault:v2:opaque","key_version":"2"}),
        json!({"ciphertext":"vault:v2:opaque","key_version":0}),
        json!({"ciphertext":"vault:v2:opaque","key_version":2.5}),
        json!({"ciphertext":"vault:v2147483648:opaque","key_version":2147483648_u64}),
    ] {
        assert!(matches!(
            encrypted(
                &profile,
                &key,
                EncryptionVersion::Latest,
                &mut json!({"data":data})
            ),
            Err(Error::Protocol)
        ));
    }
    let mut response = json!({"data":{"ciphertext":"vault:v3:opaque","key_version":3}});
    assert!(matches!(
        encrypted(
            &profile,
            &key,
            EncryptionVersion::pinned(2).unwrap(),
            &mut response
        ),
        Err(Error::Protocol)
    ));
    for selection in [
        EncryptionVersion::Latest,
        EncryptionVersion::pinned(2).unwrap(),
    ] {
        let mut response = json!({"data":{"ciphertext":"vault:v2:opaque","key_version":2}});
        assert_eq!(
            encrypted(&profile, &key, selection, &mut response)
                .unwrap()
                .version(),
            2
        );
    }
}
