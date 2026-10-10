use crate::{
    Binding, Counts, Error, Failure, Inventory, Limits, Phase,
    model::{same_blob, validate_row},
    observer::Observer,
    publication::read_private,
};
use rom::{Resource, Runtime};
use rom_blob::{Blob, BlobService, BlobState, BlobStore, Digest};
use rom_blob_recovery::{Context, Entry};
use rom_extras_maintenance::Backend;
use std::{collections::BTreeMap, path::Path, sync::Arc};
/// Isolated archive-derived inspection. Native preparation is synchronous; no Runtime or observer is exposed.
pub struct Inspection {
    runtime: Runtime,
    stores: BTreeMap<String, Arc<dyn BlobStore>>,
    rows: Vec<ReadyRow>,
    counts: Counts,
    checkpoint: Digest,
    backend: Backend,
    limits: Limits,
}
pub(crate) struct ReadyRow {
    pub(crate) id: String,
    pub(crate) revision: u64,
    pub(crate) blob: Blob,
}
impl Inspection {
    /// Validate a trusted immutable archive and restore an explicit fresh inspection database.
    /// Preserve the database after every outcome. Use a blocking host context; no native hard deadline is promised.
    pub fn prepare(
        backend: Backend,
        archive: &Path,
        inspection_database: &Path,
        stores: Vec<(String, Arc<dyn BlobStore>)>,
        limits: Limits,
    ) -> Result<Self, Error> {
        let enabled = match backend {
            Backend::Sqlite => cfg!(feature = "sqlite"),
            Backend::Redb => cfg!(feature = "redb"),
        };
        if !enabled {
            return Err(Error::Unsupported);
        }
        limits.validate()?;
        if stores.is_empty() || stores.len() > 32 {
            return Err(Error::Invalid);
        }
        let mut selected = BTreeMap::new();
        for (alias, store) in stores {
            if alias.is_empty() {
                return Err(Error::Invalid);
            }
            if alias.len() > 64 {
                return Err(Error::TooLarge);
            }
            if selected.insert(alias, store).is_some() {
                return Err(Error::Invalid);
            }
        }
        let before = Digest::of(&read_private(archive, limits.archive.max_bytes)?);
        let (_, snapshot) =
            rom_backup::read(archive, backend, limits.archive).map_err(Error::from)?;
        let mut counts = Counts::default();
        let mut rows = Vec::new();
        let mut total = 0u64;
        for row in snapshot.rows {
            if row.key.kind != Blob::KIND {
                counts.other_rows += 1;
                continue;
            }
            let Some(value) = row.value else {
                counts.tombstones += 1;
                continue;
            };
            let blob = Blob::decode(value).map_err(Error::from)?;
            match blob.state {
                BlobState::Pending => {
                    counts.pending += 1;
                    continue;
                }
                BlobState::Detached => {
                    counts.detached += 1;
                    continue;
                }
                BlobState::Ready => counts.ready += 1,
            }
            validate_row(&row.key.id, row.revision, &blob, limits)?;
            if !selected.contains_key(&blob.store) {
                return Err(Error::Invalid);
            }
            if counts.ready > limits.max_ready {
                return Err(Error::TooLarge);
            }
            total = total.checked_add(blob.bytes).ok_or(Error::TooLarge)?;
            if total > limits.max_total_bytes {
                return Err(Error::TooLarge);
            }
            rows.push(ReadyRow {
                id: row.key.id,
                revision: row.revision,
                blob,
            });
        }
        crate::codec::check_plan_size(backend, &before, counts, &rows, limits.max_json_bytes)?;
        let native_limits = rom_extras_maintenance::Limits::new(
            limits.archive.max_bytes,
            limits.archive.max_records,
        )
        .map_err(Error::from)?;
        let storage =
            rom_extras_maintenance::restore(backend, archive, inspection_database, native_limits)
                .map_err(Error::from)?;
        let after = Digest::of(&read_private(archive, limits.archive.max_bytes)?);
        if before != after {
            return Err(Error::Integrity);
        }
        let runtime = Runtime::builder()
            .resource(rom_blob::definition())
            .build(storage, Runtime::shared_cpu_pool(2).map_err(Error::from)?)
            .map_err(Error::from)?;
        Ok(Self {
            runtime,
            stores: selected,
            rows,
            counts,
            checkpoint: before,
            backend,
            limits,
        })
    }
    /// Observe every current Ready Blob through an isolated read-only public service, then drain and shut down.
    /// This consumes the collector. Aborted observation state is never reused or exposed as a partial inventory.
    pub async fn collect(self, context: Context) -> Result<Inventory, Failure> {
        let context = Arc::new(context);
        let observers: BTreeMap<_, _> = self
            .stores
            .into_iter()
            .map(|(alias, store)| (alias, Arc::new(Observer::new(store, context.clone()))))
            .collect();
        let mut builder = BlobService::builder(self.runtime.clone());
        for (alias, store) in &observers {
            builder = builder.store(alias, store.clone());
        }
        let bound = self.limits.max_object_bytes.max(1);
        let service = builder
            .limits(rom_blob::Limits {
                operations: 1,
                blob_bytes: bound,
                chunk_bytes: bound.min(65536),
                ..rom_blob::Limits::default()
            })
            .build()
            .map_err(|e| fail(0, Phase::Read, e.into()))?;
        let result: Result<Vec<Binding>, Failure> = async {
            let mut bindings = Vec::new();
            for (index, binding) in self.rows.iter().enumerate() {
                context
                    .check()
                    .map_err(|e| fail(index, Phase::Read, e.into()))?;
                check_row(&self.runtime, binding, &context)
                    .await
                    .map_err(|e| fail(index, Phase::Row, e))?;
                let observer = &observers[&binding.blob.store];
                observer
                    .begin(&binding.blob)
                    .map_err(|e| fail(index, Phase::Observe, e))?;
                context
                    .wait_result(service.read(&rom_blob::worker_actor(), &binding.id))
                    .await
                    .map_err(|e| fail(index, Phase::Read, e.into()))?
                    .map_err(|e| {
                        fail(
                            index,
                            Phase::Read,
                            context
                                .check()
                                .err()
                                .map(Error::from)
                                .unwrap_or_else(|| e.into()),
                        )
                    })?;
                let key = observer
                    .finish()
                    .map_err(|e| fail(index, Phase::Observe, e))?;
                check_row(&self.runtime, binding, &context)
                    .await
                    .map_err(|e| fail(index, Phase::Row, e))?;
                bindings.push(Binding {
                    id: binding.id.clone(),
                    revision: binding.revision,
                    blob: binding.blob.clone(),
                    entry: Entry::new(key, binding.blob.digest.clone(), binding.blob.bytes),
                });
            }
            context
                .check()
                .map_err(|e| fail(self.rows.len(), Phase::Read, e.into()))?;
            Ok(bindings)
        }
        .await;
        // Accepted service work can survive a canceled waiter. Its per-invocation observers remain isolated until drain.
        let cleanup = service
            .shutdown()
            .await
            .map_err(Error::from)
            .and(self.runtime.shutdown().await.map_err(Error::from));
        let bindings = match result {
            Ok(b) => b,
            Err(mut error) => {
                error.cleanup = cleanup.err();
                return Err(error);
            }
        };
        cleanup.map_err(|e| fail(self.rows.len(), Phase::Shutdown, e))?;
        Inventory::new(
            self.backend,
            self.checkpoint,
            self.counts,
            bindings,
            self.limits,
        )
        .map_err(|e| fail(self.counts.ready, Phase::Observe, e))
    }
}
fn fail(index: usize, phase: Phase, cause: Error) -> Failure {
    Failure {
        verified: index,
        index,
        phase,
        cause,
        cleanup: None,
    }
}
async fn check_row(runtime: &Runtime, binding: &ReadyRow, context: &Context) -> Result<(), Error> {
    let row = context
        .wait_result(async {
            runtime
                .read::<Blob>(&rom_blob::worker_actor(), &binding.id)
                .await
                .map_err(rom_blob::Error::from)
        })
        .await
        .map_err(Error::from)?
        .map_err(Error::from)?;
    if row.revision != binding.revision
        || row
            .value
            .as_ref()
            .is_none_or(|b| !same_blob(b, &binding.blob))
    {
        return Err(Error::Integrity);
    }
    context.check().map_err(Error::from)
}
