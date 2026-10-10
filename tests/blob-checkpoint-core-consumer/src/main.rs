use rom_blob_checkpoint::{Error, Inspection, Limits};
use rom_extras_maintenance::Backend;
fn main() {
    let archive = std::path::Path::new("/nonexistent-blob-checkpoint-core-canary");
    for backend in [Backend::Sqlite, Backend::Redb] {
        assert!(matches!(
            Inspection::prepare(backend, archive, archive, Vec::new(), Limits::default()),
            Err(Error::Unsupported)
        ));
    }
    assert!(!archive.exists());
    println!(
        "Independent no-native consumer: disabled inspection rejected before filesystem access"
    );
}
