//! Bounded GeoJSON route line geometry.
use crate::{Coordinate, Error, Result};
/// One or more validated GeoJSON line strings, with at most 8192 total vertices.
pub struct RouteGeometry {
    lines: Vec<Vec<Coordinate>>,
}
impl RouteGeometry {
    /// Validate a LineString with at least two positions.
    pub fn line_string(coordinates: Vec<Coordinate>) -> Result<Self> {
        Self::multi_line_string(vec![coordinates])
    }
    /// Validate a nonempty MultiLineString; each component has at least two positions.
    pub fn multi_line_string(lines: Vec<Vec<Coordinate>>) -> Result<Self> {
        if lines.is_empty() || lines.iter().any(|line| line.len() < 2) {
            return Err(Error::InvalidGeometry);
        }
        let mut vertices = 0usize;
        for line in &lines {
            vertices = vertices.checked_add(line.len()).ok_or(Error::TooLarge)?;
            if vertices > 8192 {
                return Err(Error::TooLarge);
            }
        }
        Ok(Self { lines })
    }
    /// Encode validated longitude-first positions without provider-specific polyline formats.
    pub fn to_geojson(&self) -> serde_json::Value {
        if self.lines.len() == 1 {
            serde_json::json!({"type":"LineString","coordinates":self.lines[0].iter().map(|c|c.longitude_latitude()).collect::<Vec<_>>()})
        } else {
            serde_json::json!({"type":"MultiLineString","coordinates":self.lines.iter().map(|line|line.iter().map(|c|c.longitude_latitude()).collect::<Vec<_>>()).collect::<Vec<_>>()})
        }
    }
}

/// Explicit requested travel mode; an adapter must match its prepared routing profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TravelMode {
    /// Motor vehicle routing.
    Driving,
    /// Bicycle routing.
    Cycling,
    /// Pedestrian routing.
    Walking,
}
/// Bounded validated route query with explicit travel mode.
pub struct RouteQuery {
    waypoints: Vec<Coordinate>,
    mode: TravelMode,
}
impl RouteQuery {
    /// Admit 2 through 25 validated positions without changing their order.
    pub fn new(waypoints: Vec<Coordinate>, mode: TravelMode) -> Result<Self> {
        if waypoints.len() > 25 {
            return Err(Error::TooLarge);
        }
        if waypoints.len() < 2 {
            return Err(Error::InvalidQuery);
        }
        Ok(Self { waypoints, mode })
    }
    /// Exact requested waypoint order.
    pub fn waypoints(&self) -> &[Coordinate] {
        &self.waypoints
    }
    /// Explicit requested travel mode.
    pub fn travel_mode(&self) -> TravelMode {
        self.mode
    }
}
/// Provider-derived route with explicit distance, estimated duration and provenance.
pub struct Route {
    geometry: RouteGeometry,
    distance: crate::Metres,
    duration: crate::Seconds,
    provenance: crate::Provenance,
}
impl Route {
    /// Compose already validated quantities; this grants no Resource access.
    pub fn new(
        geometry: RouteGeometry,
        distance: crate::Metres,
        duration: crate::Seconds,
        provenance: crate::Provenance,
    ) -> Self {
        Self {
            geometry,
            distance,
            duration,
            provenance,
        }
    }
    /// Validated longitude-first route geometry.
    pub fn geometry(&self) -> &RouteGeometry {
        &self.geometry
    }
    /// Total route distance in metres.
    pub fn distance(&self) -> crate::Metres {
        self.distance
    }
    /// Provider-estimated duration in seconds; not live traffic evidence.
    pub fn duration(&self) -> crate::Seconds {
        self.duration
    }
    /// Provider/data attribution and original route identity.
    pub fn provenance(&self) -> &crate::Provenance {
        &self.provenance
    }
}
