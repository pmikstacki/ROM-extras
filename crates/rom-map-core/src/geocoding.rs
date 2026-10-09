//! Bounded geocoding inputs and explicitly qualified result accuracy.
use crate::{BoundingBox, Coordinate, Error, Result};
/// Immutable bounded free-form geocoding query.
pub struct GeocodeQuery {
    text: String,
    limit: usize,
    bounds: Option<BoundingBox>,
}
impl GeocodeQuery {
    /// Admit nonempty text up to 1024 UTF-8 bytes and 1 through 40 results.
    pub fn new(text: &str, limit: usize) -> Result<Self> {
        validate_text(text, 1024, Error::InvalidQuery)?;
        validate_limit(limit)?;
        Ok(Self {
            text: text.into(),
            limit,
            bounds: None,
        })
    }
    /// Add explicit geographic bounds without changing query text.
    pub fn with_bounds(mut self, bounds: BoundingBox) -> Self {
        self.bounds = Some(bounds);
        self
    }
    /// Exact admitted text; never included in error messages.
    pub fn text(&self) -> &str {
        &self.text
    }
    /// Maximum returned results; adapters can enforce stricter provider limits before I/O.
    pub fn limit(&self) -> usize {
        self.limit
    }
    /// Optional explicitly selected bounds.
    pub fn bounds(&self) -> Option<BoundingBox> {
        self.bounds
    }
}
/// Provider-reported accuracy, never inferred from relevance, importance or result count.
pub enum Accuracy {
    /// No measured positional accuracy was supplied.
    Unknown,
    /// Explicit provider-reported uncertainty in metres.
    Radius(crate::Metres),
}
/// Attribution as plain text and an optional explicitly selected HTTP(S) credit link.
pub struct Attribution {
    text: String,
    link: Option<String>,
}
impl Attribution {
    /// Bound required plain text and reject credential-bearing or non-HTTP credit links.
    pub fn new(text: &str, link: Option<&str>) -> Result<Self> {
        validate_text(text, 1024, Error::InvalidResponse)?;
        if text.contains(['<', '>']) {
            return Err(Error::InvalidResponse);
        }
        if let Some(link) = link {
            if link.len() > 2048 {
                return Err(Error::TooLarge);
            }
            let url = url::Url::parse(link).map_err(|_| Error::InvalidResponse)?;
            if !matches!(url.scheme(), "https" | "http")
                || url.host().is_none()
                || !url.username().is_empty()
                || url.password().is_some()
                || url.query().is_some()
            {
                return Err(Error::InvalidResponse);
            }
        }
        Ok(Self {
            text: text.into(),
            link: link.map(str::to_owned),
        })
    }
    /// Required plain text credit. Render as text, never as untrusted HTML.
    pub fn text(&self) -> &str {
        &self.text
    }
    /// Explicit credit link; no network connection is initiated by this value.
    pub fn link(&self) -> Option<&str> {
        self.link.as_deref()
    }
}
/// Native provider and original result identity with mandatory attribution.
pub struct Provenance {
    provider: String,
    source_id: String,
    attribution: Attribution,
}
impl Provenance {
    /// Bound provider and original source identity without normalization or invented Resource IDs.
    pub fn new(provider: &str, source_id: &str, attribution: Attribution) -> Result<Self> {
        validate_text(provider, 128, Error::InvalidResponse)?;
        validate_text(source_id, 4096, Error::InvalidResponse)?;
        Ok(Self {
            provider: provider.into(),
            source_id: source_id.into(),
            attribution,
        })
    }
    /// Provider identity, distinct from a ROM Resource kind.
    pub fn provider(&self) -> &str {
        &self.provider
    }
    /// Exact original native result identifier.
    pub fn source_id(&self) -> &str {
        &self.source_id
    }
    /// Required provider/dataset credit.
    pub fn attribution(&self) -> &Attribution {
        &self.attribution
    }
}
/// One validated provider result, not a ROM Resource mutation or authorization grant.
pub struct GeocodeResult {
    label: String,
    coordinate: Coordinate,
    accuracy: Accuracy,
    provenance: Provenance,
    bounds: Option<BoundingBox>,
}
impl GeocodeResult {
    /// Validate bounded result text and preserve explicit accuracy and provenance.
    pub fn new(
        label: &str,
        coordinate: Coordinate,
        accuracy: Accuracy,
        provenance: Provenance,
        bounds: Option<BoundingBox>,
    ) -> Result<Self> {
        validate_text(label, 4096, Error::InvalidResponse)?;
        Ok(Self {
            label: label.into(),
            coordinate,
            accuracy,
            provenance,
            bounds,
        })
    }
    /// Plain display label.
    pub fn label(&self) -> &str {
        &self.label
    }
    /// Validated longitude-first result position.
    pub fn coordinate(&self) -> Coordinate {
        self.coordinate
    }
    /// Explicit positional accuracy, possibly unknown.
    pub fn accuracy(&self) -> &Accuracy {
        &self.accuracy
    }
    /// Original provider attribution and identity.
    pub fn provenance(&self) -> &Provenance {
        &self.provenance
    }
    /// Geographic feature extent; this is not a measured uncertainty radius.
    pub fn bounds(&self) -> Option<BoundingBox> {
        self.bounds
    }
}
/// Bounded results; an empty successful response is valid.
pub struct GeocodeResults {
    results: Vec<GeocodeResult>,
}
impl GeocodeResults {
    /// Reject excess results rather than truncate, with an explicit admitted query limit.
    pub fn new(results: Vec<GeocodeResult>, limit: usize) -> Result<Self> {
        validate_limit(limit)?;
        if results.len() > limit {
            return Err(Error::TooLarge);
        }
        Ok(Self { results })
    }
    /// Approved bounded native results.
    pub fn results(&self) -> &[GeocodeResult] {
        &self.results
    }
}
fn validate_limit(limit: usize) -> Result<()> {
    if !(1..=40).contains(&limit) {
        Err(Error::InvalidQuery)
    } else {
        Ok(())
    }
}
fn validate_text(text: &str, limit: usize, error: Error) -> Result<()> {
    if text.len() > limit {
        return Err(Error::TooLarge);
    }
    if text.trim().is_empty() || text.chars().any(char::is_control) {
        return Err(error);
    }
    Ok(())
}
