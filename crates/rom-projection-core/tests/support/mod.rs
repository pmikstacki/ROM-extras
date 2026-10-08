// Integration binaries use different subsets of these shared fixture helpers.
#![allow(dead_code)]
use rom::JournalCursor;
use rom_projection_core::{Checkpoint, ProjectionProfile};
mod fs;
#[allow(unused_imports)] // Boundary-only integration binaries do not create native files.
pub use fs::Directory;

pub fn profile() -> ProjectionProfile {
    ProjectionProfile::new("deployment-a", "qdrant", "mapping-v1", None).unwrap()
}

pub fn cursor(position: u64) -> JournalCursor {
    JournalCursor {
        generation: "journal-a".into(),
        kind: "document".into(),
        position,
    }
}

pub fn initial() -> Checkpoint {
    Checkpoint::new("physical-a", vec![cursor(0)]).unwrap()
}
