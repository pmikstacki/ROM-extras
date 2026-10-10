use crate::{Cause, Context, Entry, Failure, Manifest, Phase, Publication, Report};
use rom_blob::{BlobStore, Digest};

fn failure(
    progress: Report,
    index: usize,
    phase: Phase,
    cause: Cause,
    publication: Publication,
) -> Failure {
    Failure {
        progress,
        index,
        phase,
        cause,
        publication,
    }
}
async fn read(store: &dyn BlobStore, e: &Entry, c: &Context) -> Result<Vec<u8>, Cause> {
    c.check()?;
    let bytes = c.wait(store.get(e.key(), e.bytes() as usize)).await?;
    c.check()?;
    if bytes.len() as u64 != e.bytes() || Digest::of(&bytes) != *e.digest() {
        return Err(Cause::Integrity);
    }
    Ok(bytes)
}
/// Copy and read-back verify a complete validated inventory, sequentially, without overwrites.
/// Existing equal content is an explicit resume. Each absent entry admits at most one create.
/// No delete or automatic retry occurs. Host approval and source retention are prerequisites.
pub async fn copy(
    source: &dyn BlobStore,
    destination: &dyn BlobStore,
    manifest: &Manifest,
    context: &Context,
) -> Result<Report, Failure> {
    let mut progress = Report::default();
    for (index, entry) in manifest.entries().iter().enumerate() {
        match read(destination, entry, context).await {
            Ok(_) => {
                progress.existing += 1;
            }
            Err(Cause::Missing) => {
                let bytes = read(source, entry, context).await.map_err(|e| {
                    failure(
                        progress,
                        index,
                        Phase::SourceRead,
                        e,
                        Publication::NotAttempted,
                    )
                })?;
                context.check().map_err(|e| {
                    failure(progress, index, Phase::Create, e, Publication::NotAttempted)
                })?;
                let publication = match context.create(destination.create(entry.key(), bytes)).await
                {
                    Ok(()) => Publication::Confirmed,
                    Err(rom_blob::Error::Conflict) => Publication::Conflict,
                    Err(error) => {
                        let cause = context.check().err().unwrap_or_else(|| Cause::from(error));
                        return Err(failure(
                            progress,
                            index,
                            Phase::Create,
                            cause,
                            Publication::Unknown,
                        ));
                    }
                };
                read(destination, entry, context).await.map_err(|e| {
                    failure(progress, index, Phase::DestinationVerify, e, publication)
                })?;
                if publication == Publication::Confirmed {
                    progress.created += 1;
                } else {
                    progress.existing += 1;
                }
            }
            Err(e) => {
                return Err(failure(
                    progress,
                    index,
                    Phase::DestinationRead,
                    e,
                    Publication::NotAttempted,
                ));
            }
        }
        progress.verified += 1;
        progress.bytes += entry.bytes();
    }
    context.check().map_err(|e| {
        failure(
            progress,
            manifest.entries().len(),
            Phase::DestinationRead,
            e,
            Publication::NotAttempted,
        )
    })?;
    Ok(progress)
}
/// Read and verify supplied entries without any publication or deletion.
/// Success is per-entry content evidence, not a coherent provider-wide snapshot.
pub async fn verify(
    store: &dyn BlobStore,
    manifest: &Manifest,
    context: &Context,
) -> Result<Report, Failure> {
    let mut progress = Report::default();
    for (index, entry) in manifest.entries().iter().enumerate() {
        read(store, entry, context).await.map_err(|e| {
            failure(
                progress,
                index,
                Phase::DestinationRead,
                e,
                Publication::NotAttempted,
            )
        })?;
        progress.verified += 1;
        progress.bytes += entry.bytes();
        progress.existing += 1;
    }
    context.check().map_err(|e| {
        failure(
            progress,
            manifest.entries().len(),
            Phase::DestinationRead,
            e,
            Publication::NotAttempted,
        )
    })?;
    Ok(progress)
}
