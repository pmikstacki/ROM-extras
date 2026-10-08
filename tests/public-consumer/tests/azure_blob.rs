//! Independent public Azure adapter consumption against an explicitly scoped emulator.
#[path = "../../common/azure_fixture.rs"]
mod fixture;
use rom_blob::{BlobStore, Digest, Error, ObjectKey};
#[tokio::test]
async fn public_consumer_preserves_false_and_create_only_contract() {
    let store = fixture::emulator(1024);
    let identity = format!(
        "consumer-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let key = ObjectKey::parse(Digest::of(identity.as_bytes()).as_str()).unwrap();
    store.create(&key, b"false".to_vec()).await.unwrap();
    assert_eq!(
        store.create(&key, b"true".to_vec()).await,
        Err(Error::Conflict)
    );
    assert_eq!(store.head(&key).await.unwrap().bytes, 5);
    assert_eq!(store.get(&key, 5).await.unwrap(), b"false");
    store.delete(&key).await.unwrap();
    assert_eq!(store.get(&key, 5).await, Err(Error::Missing));
}
