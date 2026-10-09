//! Typed map capabilities outside storage, projections and browser rendering.
mod coordinates;
mod error;
mod routing;
mod units;
pub use coordinates::{BoundingBox, Coordinate};
pub use error::{Error, Result};
pub use routing::RouteGeometry;
pub use units::{Metres, Seconds};
mod geocoding;
mod resource;
pub use geocoding::{
    Accuracy, Attribution, GeocodeQuery, GeocodeResult, GeocodeResults, Provenance,
};
pub use resource::ResourceLocation;
mod capabilities;
mod context;
pub use capabilities::{Capabilities, Geocoding, ProviderFuture, ReverseGeocoding, Routing};
pub use context::{Cancellation, RequestContext};
pub use routing::{Route, RouteQuery, TravelMode};
mod sources;
pub use capabilities::{RasterTiles, Styles, VectorTiles};
pub use sources::{
    BrowserPolicy, MapStyle, RasterSource, RasterTileSize, TileKind, TileSource, VectorSource,
};
