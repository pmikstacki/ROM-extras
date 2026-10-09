//! WGS84 longitude-first positions and antimeridian-aware bounds.
use crate::{Error, Result};
/// Validated longitude, latitude in decimal degrees.
#[derive(Clone, Copy, PartialEq)]
pub struct Coordinate {
    longitude: f64,
    latitude: f64,
}
impl Coordinate {
    /// Validate longitude in [-180,180] followed by latitude in [-90,90].
    pub fn new(longitude: f64, latitude: f64) -> Result<Self> {
        if !longitude.is_finite()
            || !latitude.is_finite()
            || !(-180.0..=180.0).contains(&longitude)
            || !(-90.0..=90.0).contains(&latitude)
        {
            return Err(Error::InvalidCoordinate);
        }
        Ok(Self {
            longitude,
            latitude,
        })
    }
    /// WGS84 longitude in decimal degrees.
    pub fn longitude_degrees(self) -> f64 {
        self.longitude
    }
    /// WGS84 latitude in decimal degrees.
    pub fn latitude_degrees(self) -> f64 {
        self.latitude
    }
    /// GeoJSON position order, longitude then latitude.
    pub fn longitude_latitude(self) -> [f64; 2] {
        [self.longitude, self.latitude]
    }
}
impl std::fmt::Debug for Coordinate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Coordinate")
    }
}
/// Validated west, south, east, north; west > east crosses the antimeridian.
#[derive(Clone, Copy, PartialEq)]
pub struct BoundingBox {
    west: f64,
    south: f64,
    east: f64,
    north: f64,
}
impl BoundingBox {
    /// Validate coordinate extents without reordering an antimeridian crossing.
    pub fn new(west: f64, south: f64, east: f64, north: f64) -> Result<Self> {
        Coordinate::new(west, south).map_err(|_| Error::InvalidBounds)?;
        Coordinate::new(east, north).map_err(|_| Error::InvalidBounds)?;
        if south > north {
            return Err(Error::InvalidBounds);
        }
        Ok(Self {
            west,
            south,
            east,
            north,
        })
    }
    /// GeoJSON bounds order.
    pub fn west_south_east_north(self) -> [f64; 4] {
        [self.west, self.south, self.east, self.north]
    }
    /// True for a box whose western longitude exceeds its eastern longitude.
    pub fn crosses_antimeridian(self) -> bool {
        self.west > self.east
    }
    /// Test inclusion with explicit antimeridian semantics.
    pub fn contains(self, coordinate: Coordinate) -> bool {
        let lon = coordinate.longitude;
        let longitude_inside = if self.crosses_antimeridian() {
            lon >= self.west || lon <= self.east
        } else {
            (self.west..=self.east).contains(&lon)
        };
        longitude_inside && (self.south..=self.north).contains(&coordinate.latitude)
    }
}
impl std::fmt::Debug for BoundingBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BoundingBox")
    }
}
