use rom_map_core::{
    Error, GeocodeQuery, GeocodeResults, Geocoding, ProviderFuture, RequestContext, Result,
};
use std::time::Duration;
/// Explicit private service configuration; public OSMF infrastructure is not admitted.
pub struct Config {
    pub(crate) http: rom_map_http::Config,
    pub(crate) provider: String,
}
impl Config {
    /// Validate an operator-selected endpoint, identifying agent, provenance and dispatch interval.
    pub fn new(endpoint: &str, agent: &str, provider: &str, interval: Duration) -> Result<Self> {
        let url = url::Url::parse(endpoint).map_err(|_| Error::InvalidQuery)?;
        if url.host_str().is_some_and(|host| {
            host.trim_end_matches('.')
                .eq_ignore_ascii_case("nominatim.openstreetmap.org")
        }) {
            return Err(Error::Rejected);
        }
        if provider.trim().is_empty()
            || provider.len() > 128
            || provider.chars().any(char::is_control)
        {
            return Err(Error::InvalidQuery);
        }
        Ok(Self {
            http: rom_map_http::Config::new(endpoint, agent, 1048576, interval, 1)?,
            provider: provider.into(),
        })
    }
    /// Use a private service's approved certificate authority.
    pub fn with_ca(mut self, ca: Vec<u8>) -> Result<Self> {
        self.http = self.http.with_ca(ca)?;
        Ok(self)
    }
}
/// Nominatim search capability; no implicit service, cache, browser grant or ROM mutation.
pub struct Nominatim {
    http: rom_map_http::Http,
    provider: String,
}
impl Nominatim {
    /// Prepare the adapter without making a connection.
    pub fn new(config: Config) -> Result<Self> {
        Ok(Self {
            http: rom_map_http::Http::new(config.http)?,
            provider: config.provider,
        })
    }
}
impl Geocoding for Nominatim {
    fn geocode<'a>(
        &'a self,
        query: &'a GeocodeQuery,
        context: &'a RequestContext,
    ) -> ProviderFuture<'a, GeocodeResults> {
        Box::pin(async move {
            context
                .run(async {
                    let limit = query.limit().to_string();
                    let mut params = vec![
                        ("q", query.text()),
                        ("format", "jsonv2"),
                        ("limit", limit.as_str()),
                    ];
                    let viewbox;
                    if let Some(bounds) = query.bounds() {
                        if bounds.crosses_antimeridian() {
                            return Err(Error::Unsupported);
                        }
                        let [west, south, east, north] = bounds.west_south_east_north();
                        viewbox = format!("{west},{south},{east},{north}");
                        params.push(("viewbox", viewbox.as_str()));
                        params.push(("bounded", "1"));
                    }
                    let bytes = self.http.get(&["search"], &params, context).await?;
                    crate::search_results(&bytes, query.limit(), &self.provider)
                })
                .await
        })
    }
}
impl rom_map_core::ReverseGeocoding for Nominatim {
    fn reverse<'a>(
        &'a self,
        coordinate: rom_map_core::Coordinate,
        context: &'a RequestContext,
    ) -> ProviderFuture<'a, GeocodeResults> {
        Box::pin(async move {
            context
                .run(async {
                    let longitude = coordinate.longitude_degrees().to_string();
                    let latitude = coordinate.latitude_degrees().to_string();
                    let bytes = self
                        .http
                        .get(
                            &["reverse"],
                            &[
                                ("lon", longitude.as_str()),
                                ("lat", latitude.as_str()),
                                ("format", "jsonv2"),
                                ("addressdetails", "0"),
                            ],
                            context,
                        )
                        .await?;
                    crate::reverse_results(&bytes, &self.provider)
                })
                .await
        })
    }
}
