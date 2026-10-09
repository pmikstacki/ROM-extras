//! Public authorized historical feeds with fixed host mapping/model, without storage internals.
use crate::{
    ApprovedDocument, DocumentMapping, Error, ProjectionHistory, ProjectionProfile, Result, limits,
};
use rom::{Actor, JournalBatch, JournalCursor, JournalView, Key, Runtime, Value};
/// Bounded selected authorized fields for a pure host vector lookup or computation.
/// No unselected Resource field, storage row, current read or journal position is exposed.
pub struct VectorInput<'a> {
    key: &'a Key,
    revision: u64,
    fields: &'a [(&'a str, &'a Value)],
}
impl<'a> VectorInput<'a> {
    /// Original Resource identity.
    pub fn key(&self) -> &'a Key {
        self.key
    }
    /// Exact historical Resource revision.
    pub fn revision(&self) -> u64 {
        self.revision
    }
    /// Selected fields, already checked against canonical value bounds.
    pub fn fields(&self) -> &'a [(&'a str, &'a Value)] {
        self.fields
    }
}
/// Trusted pure, bounded host provider; no I/O, reentrant runtime calls or unbounded allocation.
/// The immutable model identity must describe the deterministic operation and remain unchanged.
/// Return the configured finite vector from already available host data or bounded computation.
pub trait HostVectors: Send {
    /// Fixed model identity from the durable profile.
    fn model_id(&self) -> &str;
    /// Produce one vector from bounded selected historical input. Tombstones never call this.
    fn vector(&mut self, input: VectorInput<'_>) -> Result<Vec<f32>>;
}
/// Scalar source marker; no host vector callback is used.
pub struct NoVectors;
impl HostVectors for NoVectors {
    fn model_id(&self) -> &str {
        ""
    }
    fn vector(&mut self, _: VectorInput<'_>) -> Result<Vec<f32>> {
        Err(Error::Unsupported)
    }
}
/// Fixed public Runtime/actor/kind source. Every fetch uses Runtime::journal authorization.
/// This does not grant export authority or supply continuous remote authorization fencing.
/// The host must invalidate generations when export policy/model configuration changes.
pub struct RuntimeHistory<V: HostVectors = NoVectors> {
    runtime: Runtime,
    actor: Actor,
    kind: String,
    mapping: DocumentMapping,
    vectors: V,
}
impl RuntimeHistory<NoVectors> {
    /// Bind a scalar mapping to one fixed host-established actor and Resource kind.
    pub fn new(
        runtime: Runtime,
        actor: Actor,
        kind: &str,
        mapping: DocumentMapping,
    ) -> Result<Self> {
        if mapping.vector_dimension().is_some() {
            return Err(Error::Unsupported);
        }
        Self::bind(runtime, actor, kind, mapping, NoVectors)
    }
}
impl<V: HostVectors> RuntimeHistory<V> {
    /// Bind a vector mapping and pure host provider with the exact configured model identity.
    pub fn with_vectors(
        runtime: Runtime,
        actor: Actor,
        kind: &str,
        mapping: DocumentMapping,
        vectors: V,
    ) -> Result<Self> {
        if mapping.vector_dimension().is_none() {
            return Err(Error::Invalid);
        }
        limits::identifier(vectors.model_id(), limits::IDENTIFIER)?;
        Self::bind(runtime, actor, kind, mapping, vectors)
    }
    fn bind(
        runtime: Runtime,
        actor: Actor,
        kind: &str,
        mapping: DocumentMapping,
        vectors: V,
    ) -> Result<Self> {
        for value in [kind, actor.authority.as_str(), actor.subject.as_str()]
            .into_iter()
            .chain(actor.host_stamp())
        {
            limits::identifier(value, limits::IDENTIFIER)?;
        }
        let source = Self {
            runtime,
            actor,
            kind: kind.into(),
            mapping,
            vectors,
        };
        source.check_model()?;
        Ok(source)
    }
    fn check_model(&self) -> Result<()> {
        if self.mapping.vector_dimension().is_some()
            && self.mapping.profile().model.as_deref() != Some(self.vectors.model_id())
        {
            return Err(Error::RebuildRequired);
        }
        Ok(())
    }
}
impl<V: HostVectors> ProjectionHistory for RuntimeHistory<V> {
    fn profile(&self) -> &ProjectionProfile {
        self.mapping.profile()
    }
    async fn fetch(&mut self, after: &JournalCursor) -> Result<JournalBatch> {
        limits::cursor(after)?;
        if after.kind != self.kind {
            return Err(Error::Invalid);
        }
        self.check_model()?;
        let result = self
            .runtime
            .journal(&self.actor, &self.kind, Some(after))
            .await;
        self.check_model()?;
        result.map_err(source_error)
    }
    fn document(&mut self, event: &JournalView) -> Result<ApprovedDocument> {
        if event.view.key.kind != self.kind {
            return Err(Error::Invalid);
        }
        self.check_model()?;
        let vector_enabled = self.mapping.vector_dimension().is_some();
        let tombstone = event.view.value.is_none();
        let expected_model = self.mapping.profile().model.as_deref();
        self.mapping.build(event, |fields| {
            if !vector_enabled || tombstone {
                return Ok(None);
            }
            let vector = self.vectors.vector(VectorInput {
                key: &event.view.key,
                revision: event.view.revision,
                fields,
            });
            if expected_model != Some(self.vectors.model_id()) {
                return Err(Error::RebuildRequired);
            }
            vector.map(Some)
        })
    }
}
fn source_error(error: rom::Error) -> Error {
    match error {
        rom::Error::HistoryGap => Error::HistoryGap,
        rom::Error::TooLarge => Error::TooLarge,
        rom::Error::Unsupported(_) => Error::Unsupported,
        rom::Error::Denied | rom::Error::IdentityExpired | rom::Error::IdentityMismatch => {
            Error::RebuildRequired
        }
        rom::Error::Invalid { .. } | rom::Error::Unregistered => Error::Invalid,
        _ => Error::SourceUnavailable,
    }
}
