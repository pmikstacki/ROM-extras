mod cli;
mod extended;
mod interruption;
mod model;
mod native;
use rom_extras_maintenance::{Backend, Error, Limits, inspect, restore};
fn main() {
    assert_eq!(Limits::new(0, 1).unwrap_err(), Error::Invalid);
    assert_eq!(Limits::new(1, 0).unwrap_err(), Error::Invalid);
    assert_eq!(
        Limits::new(128 * 1024 * 1024 + 1, 1).unwrap_err(),
        Error::TooLarge
    );
    assert_eq!(Limits::new(1, 400001).unwrap_err(), Error::TooLarge);
    assert!(
        inspect(
            Backend::Sqlite,
            std::path::Path::new("/nonexistent-maintenance-canary"),
            Limits::default()
        )
        .is_err()
    );
    assert!(
        restore(
            Backend::Redb,
            std::path::Path::new("/nonexistent-maintenance-canary"),
            std::path::Path::new("/nonexistent-destination-canary"),
            Limits::default()
        )
        .is_err()
    );
    let root = std::path::PathBuf::from(
        std::env::var("ROM_EXTRAS_MAINTENANCE_RUN")
            .expect("explicit private retained run directory required"),
    );
    assert!(root.is_dir());
    let executor = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    executor.block_on(native::run(&root));
    extended::run(&root);
    cli::run(&root);
    interruption::run(&root);
}
