//! Bounded typed text input, private approval tokens and original-key candidates.
use crate::{Error, ProjectionProfile, Result, limits};
use rom::Key;
/// Explicit full-text term combination; providers do not choose an implicit mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextMode {
    /// Require every analyzed term.
    AllTerms,
    /// Accept any analyzed term.
    AnyTerms,
}
/// Immutable bounded text input; no provider DSL or request path.
pub struct TextQuery {
    field: String,
    text: String,
    mode: TextMode,
    limit: usize,
    budget: usize,
}
impl TextQuery {
    /// Validate bounded input without granting query authority.
    pub fn new(
        field: &str,
        text: &str,
        mode: TextMode,
        result_limit: usize,
        candidate_budget: usize,
    ) -> Result<Self> {
        limits::identifier(field, 128)?;
        if text.len() > 4096 {
            return Err(Error::TooLarge);
        }
        if text.trim().is_empty()
            || text.contains('\0')
            || result_limit == 0
            || result_limit > 64
            || candidate_budget < result_limit
            || candidate_budget > 256
        {
            return Err(Error::Invalid);
        }
        Ok(Self {
            field: field.into(),
            text: text.into(),
            mode,
            limit: result_limit,
            budget: candidate_budget,
        })
    }
    /// Requested field name.
    pub fn field(&self) -> &str {
        &self.field
    }
    /// Explicit sensitive text access; Debug omits this value.
    pub fn text(&self) -> &str {
        &self.text
    }
    /// Explicit requested text matching mode.
    pub fn mode(&self) -> TextMode {
        self.mode
    }
    /// Maximum current authorized values returned.
    pub fn result_limit(&self) -> usize {
        self.limit
    }
    /// Maximum candidates admitted from one backend response.
    pub fn candidate_budget(&self) -> usize {
        self.budget
    }
}
/// Fixed authorization scope; the host decides current kind/field/mode/generation permission.
pub struct SearchScope {
    pub(crate) kind: String,
    pub(crate) profile: ProjectionProfile,
    pub(crate) physical: String,
    pub(crate) query: TextQuery,
}
impl SearchScope {
    /// Fixed Resource kind.
    pub fn kind(&self) -> &str {
        &self.kind
    }
    /// Requested field name.
    pub fn field(&self) -> &str {
        self.query.field()
    }
    /// Explicit requested text matching mode.
    pub fn mode(&self) -> TextMode {
        self.query.mode()
    }
    /// Fixed immutable deployment and mapping profile.
    pub fn profile(&self) -> &ProjectionProfile {
        &self.profile
    }
    /// Fixed approved physical generation.
    pub fn physical_target(&self) -> &str {
        &self.physical
    }
}
/// Core-minted request. Construction does not escape the current policy check.
///
/// ```compile_fail
/// use rom_projection_core::{ApprovedTextQuery, SearchScope};
/// fn forge(scope: SearchScope) -> ApprovedTextQuery {
///     ApprovedTextQuery { scope }
/// }
/// ```
pub struct ApprovedTextQuery {
    pub(crate) scope: SearchScope,
}
impl ApprovedTextQuery {
    /// Host authorization scope bound to this request.
    pub fn scope(&self) -> &SearchScope {
        &self.scope
    }
    /// Bounded typed input; text access is explicit.
    pub fn query(&self) -> &TextQuery {
        &self.scope.query
    }
    /// Fixed immutable deployment and mapping profile.
    pub fn profile(&self) -> &ProjectionProfile {
        self.scope.profile()
    }
    /// Fixed approved physical generation.
    pub fn physical_target(&self) -> &str {
        self.scope.physical_target()
    }
}
/// Bounded original key and indexed revision; no backend value or score is retained.
pub struct SearchCandidate {
    key: Key,
    revision: u64,
}
impl SearchCandidate {
    /// Validate bounded input without granting query authority.
    pub fn new(key: Key, revision: u64) -> Result<Self> {
        limits::key(&key)?;
        if revision == 0 {
            return Err(Error::Invalid);
        }
        Ok(Self { key, revision })
    }
    /// Original bounded Resource key.
    pub fn key(&self) -> &Key {
        &self.key
    }
    /// Indexed positive Resource revision.
    pub fn revision(&self) -> u64 {
        self.revision
    }
}
crate::diagnostics::sanitized_debug!(TextQuery, SearchScope, ApprovedTextQuery, SearchCandidate);
