//! Independent dynamically selectable map operations, unrelated to SQL/projection ports.
use crate::{
    Coordinate, Error, GeocodeQuery, GeocodeResults, RequestContext, Result, Route, RouteQuery,
};
use std::{future::Future, pin::Pin};
/// Bounded provider-operation future; cancellation is supplied explicitly by the host.
pub type ProviderFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;
/// Optional free-form geocoding capability.
pub trait Geocoding: Send + Sync {
    /// Resolve an explicit host query, respecting its context and selected operator policy.
    fn geocode<'a>(
        &'a self,
        query: &'a GeocodeQuery,
        context: &'a RequestContext,
    ) -> ProviderFuture<'a, GeocodeResults>;
}
/// Optional reverse geocoding capability.
pub trait ReverseGeocoding: Send + Sync {
    /// Resolve an explicit position; never activate geolocation.
    fn reverse<'a>(
        &'a self,
        coordinate: Coordinate,
        context: &'a RequestContext,
    ) -> ProviderFuture<'a, GeocodeResults>;
}
/// Optional routing capability.
pub trait Routing: Send + Sync {
    /// Resolve an explicit route, returning None for a confirmed lack of route.
    fn route<'a>(
        &'a self,
        query: &'a RouteQuery,
        context: &'a RequestContext,
    ) -> ProviderFuture<'a, Option<Route>>;
}
/// Optional host-selected MapLibre style capability.
pub trait Styles: Send + Sync {
    /// Obtain approved style data without activating browser requests.
    fn style<'a>(&'a self, context: &'a RequestContext) -> ProviderFuture<'a, crate::MapStyle>;
}
/// Optional raster tile metadata capability, separate from vector sources.
pub trait RasterTiles: Send + Sync {
    /// Obtain approved raster metadata for the host-selected source.
    fn raster_tiles<'a>(
        &'a self,
        context: &'a RequestContext,
    ) -> ProviderFuture<'a, crate::RasterSource>;
}
/// Optional vector tile metadata capability, retaining native layer identifiers.
pub trait VectorTiles: Send + Sync {
    /// Obtain approved vector metadata for the host-selected source.
    fn vector_tiles<'a>(
        &'a self,
        context: &'a RequestContext,
    ) -> ProviderFuture<'a, crate::VectorSource>;
}
/// Provider capability selection; missing ports are explicit, without fallback network calls.
#[derive(Default)]
pub struct Capabilities<'a> {
    geocoding: Option<&'a dyn Geocoding>,
    reverse: Option<&'a dyn ReverseGeocoding>,
    routing: Option<&'a dyn Routing>,
    styles: Option<&'a dyn Styles>,
    raster: Option<&'a dyn RasterTiles>,
    vector: Option<&'a dyn VectorTiles>,
}
impl<'a> Capabilities<'a> {
    /// Start with no activated provider or network operations.
    pub fn new() -> Self {
        Self::default()
    }
    /// Select the host's free-form geocoder.
    pub fn with_geocoding(mut self, provider: &'a dyn Geocoding) -> Self {
        self.geocoding = Some(provider);
        self
    }
    /// Select the host's reverse geocoder independently.
    pub fn with_reverse_geocoding(mut self, provider: &'a dyn ReverseGeocoding) -> Self {
        self.reverse = Some(provider);
        self
    }
    /// Select the host's routing provider independently.
    pub fn with_routing(mut self, provider: &'a dyn Routing) -> Self {
        self.routing = Some(provider);
        self
    }
    /// Require the selected free-form capability without inventing a default provider.
    pub fn geocoding(&self) -> Result<&'a dyn Geocoding> {
        self.geocoding.ok_or(Error::Unsupported)
    }
    /// Require the selected reverse capability without fallback to free-form search.
    pub fn reverse_geocoding(&self) -> Result<&'a dyn ReverseGeocoding> {
        self.reverse.ok_or(Error::Unsupported)
    }
    /// Require the selected routing capability without straight-line approximation.
    pub fn routing(&self) -> Result<&'a dyn Routing> {
        self.routing.ok_or(Error::Unsupported)
    }
    /// Select a style source independently from geocoding and routing.
    pub fn with_styles(mut self, provider: &'a dyn Styles) -> Self {
        self.styles = Some(provider);
        self
    }
    /// Select raster metadata independently from vector sources.
    pub fn with_raster_tiles(mut self, provider: &'a dyn RasterTiles) -> Self {
        self.raster = Some(provider);
        self
    }
    /// Select vector metadata independently from raster sources.
    pub fn with_vector_tiles(mut self, provider: &'a dyn VectorTiles) -> Self {
        self.vector = Some(provider);
        self
    }
    /// Require an explicitly selected style capability.
    pub fn styles(&self) -> Result<&'a dyn Styles> {
        self.styles.ok_or(Error::Unsupported)
    }
    /// Require an explicitly selected raster capability.
    pub fn raster_tiles(&self) -> Result<&'a dyn RasterTiles> {
        self.raster.ok_or(Error::Unsupported)
    }
    /// Require an explicitly selected vector capability.
    pub fn vector_tiles(&self) -> Result<&'a dyn VectorTiles> {
        self.vector.ok_or(Error::Unsupported)
    }
}
