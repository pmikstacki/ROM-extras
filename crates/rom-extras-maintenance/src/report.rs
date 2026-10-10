use crate::Backend;
use serde::Serialize;
/// Aggregate validated counts; no archive values, keys or private labels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Counts {
    /// Current rows including tombstones.
    pub rows: usize,
    /// Durable operation receipts.
    pub receipts: usize,
    /// Historical events.
    pub events: usize,
    /// Stored external effect intents, not external deliveries.
    pub effects: usize,
    /// Canonical Resource descriptors.
    pub descriptors: usize,
    /// Live reference edges.
    pub references: usize,
    /// Canonical durable Work records.
    pub work: usize,
    /// Durable operator receipts.
    pub operator_receipts: usize,
}
/// Point-in-time fully validated archive facts; not authorization or a cutover permit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Report {
    /// Exact native archive backend; external SQL is not disguised as this profile.
    pub backend: Backend,
    /// Archive envelope version verified by upstream.
    pub archive_version: u32,
    /// Native format marker verified by upstream.
    pub storage_format: u32,
    /// Aggregate counts from validated complete archive.
    pub counts: Counts,
    /// External object bytes are excluded.
    pub external_blobs_included: bool,
    /// External delivery execution and provider state are excluded.
    pub external_deliveries_included: bool,
}
impl From<rom_backup::Manifest> for Report {
    fn from(m: rom_backup::Manifest) -> Self {
        Self {
            backend: m.backend,
            archive_version: m.archive_version,
            storage_format: m.storage_format,
            counts: Counts {
                rows: m.rows,
                receipts: m.receipts,
                events: m.events,
                effects: m.effects,
                descriptors: m.descriptors,
                references: m.references,
                work: m.work,
                operator_receipts: m.operator_receipts,
            },
            external_blobs_included: m.external_blobs_included,
            external_deliveries_included: m.external_deliveries_included,
        }
    }
}
/// Successful in-memory typed migration preview; no destination was published.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct MigrationPreview {
    /// Validated source archive facts.
    pub source: Report,
    /// Number of changed Resource descriptor versions.
    pub changed_schemas: usize,
    /// Historical receipts whose replay codec is pinned to an older version after conversion.
    pub historical_replay_codecs: usize,
}
