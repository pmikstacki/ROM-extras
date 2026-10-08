use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);

pub(super) fn reserve(stamp: u128) -> PathBuf {
    for _ in 0..32 {
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let directory = std::env::temp_dir().join(format!(
            "rom-extras-https-{}-{stamp}-{sequence}",
            std::process::id()
        ));
        let mut builder = std::fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        match builder.create(&directory) {
            Ok(()) => return directory,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => panic!("reserve private receiver directory: {error}"),
        }
    }
    panic!("receiver directory collision budget exhausted");
}

#[cfg(test)]
#[path = "directory_tests.rs"]
mod tests;
