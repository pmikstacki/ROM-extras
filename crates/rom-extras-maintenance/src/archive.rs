use crate::{Backend, Error, Limits, MigrationPlan, MigrationPreview, Report};
use std::{collections::BTreeMap, path::Path};
/// Fully validate a private native archive and return count-only facts. No filesystem writes.
pub fn inspect(backend: Backend, archive: &Path, limits: Limits) -> Result<Report, Error> {
    let (manifest, _) = rom_backup::read(archive, backend, limits.native).map_err(Error::from)?;
    Ok(manifest.into())
}
/// Execute the host's pure bounded converters in memory without publishing a destination.
/// The host owns callback safety; this preview does not fence a later archive or deployment.
pub fn preview_migration(
    backend: Backend,
    archive: &Path,
    plan: &MigrationPlan,
    limits: Limits,
) -> Result<MigrationPreview, Error> {
    let (manifest, snapshot) =
        rom_backup::read(archive, backend, limits.native).map_err(Error::from)?;
    let before: BTreeMap<_, _> = snapshot
        .descriptors
        .iter()
        .map(|d| (d.kind.clone(), d.version))
        .collect();
    let converted =
        rom_backup::migrate_snapshot(snapshot, plan, limits.native).map_err(Error::from)?;
    Ok(MigrationPreview {
        source: manifest.into(),
        changed_schemas: converted
            .descriptors
            .iter()
            .filter(|d| before.get(&d.kind) != Some(&d.version))
            .count(),
        historical_replay_codecs: converted
            .receipts
            .iter()
            .filter(|r| r.replay_version.is_some())
            .count(),
    })
}
