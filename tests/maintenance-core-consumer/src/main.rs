use rom_extras_maintenance::{Backend, Error, Limits, restore};
fn main() {
    let archive = std::path::Path::new("/nonexistent-maintenance-core-private-canary");
    for backend in [Backend::Sqlite, Backend::Redb] {
        assert!(matches!(
            restore(backend, archive, archive, Limits::default()),
            Err(Error::Unsupported)
        ));
    }
    assert!(!archive.exists());
    println!(
        "Independent no-native consumer: both disabled restore capabilities rejected before filesystem access"
    );
}
