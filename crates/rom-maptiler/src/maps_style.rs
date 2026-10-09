//! Style metadata with explicit native source bindings, never nested auto-fetches.
use crate::MapTilerMaps;
use rom_map_core::{Error, MapStyle, ProviderFuture, RequestContext, Result, Styles, TileSource};
use std::{collections::HashSet, sync::Arc};
/// Host-selected MapLibre-v8 map style with explicitly resolved TileJSON bindings.
pub struct MapTilerStyle {
    service: Arc<MapTilerMaps>,
    id: String,
    resolved: Vec<(String, TileSource)>,
}
impl MapTilerStyle {
    /// Bind an exact map ID and approved sources indexed by their exact native style source IDs.
    pub fn new(
        service: Arc<MapTilerMaps>,
        map_id: &str,
        resolved: Vec<(String, TileSource)>,
    ) -> Result<Self> {
        if resolved.len() > 128 {
            return Err(Error::TooLarge);
        }
        let mut ids = HashSet::new();
        for (id, _) in &resolved {
            if id.len() > 128 {
                return Err(Error::TooLarge);
            }
            if id.trim().is_empty() || id.chars().any(char::is_control) || !ids.insert(id) {
                return Err(Error::InvalidQuery);
            }
        }
        Ok(Self {
            service,
            id: super::maps_service::service_id(map_id)?,
            resolved,
        })
    }
}
impl Styles for MapTilerStyle {
    fn style<'a>(&'a self, context: &'a RequestContext) -> ProviderFuture<'a, MapStyle> {
        Box::pin(async move {
            context
                .run(async {
                    let bytes = self
                        .service
                        .http
                        .get(&["maps", &self.id, "style.json"], &[], context)
                        .await?;
                    let resolved: Vec<_> = self
                        .resolved
                        .iter()
                        .map(|(id, source)| (id.as_str(), source))
                        .collect();
                    self.service.metadata.style(&bytes, &resolved)
                })
                .await
        })
    }
}
