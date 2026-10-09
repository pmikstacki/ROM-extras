use rom_map_core::{
    Error, Provenance, ProviderFuture, RequestContext, Result, Route, RouteQuery, Routing,
    TravelMode,
};
use std::time::Duration;
/// Host-selected endpoint and binding to the graph's statically prepared travel profile.
pub struct Config {
    pub(crate) http: rom_map_http::Config,
    pub(crate) profile: String,
    pub(crate) mode: TravelMode,
    pub(crate) graph: Provenance,
    pub(crate) snap_radius: Option<String>,
}
impl Config {
    /// Select a service, native URL profile and the graph's actual prepared travel mode.
    /// Changing the URL label cannot change the graph's preprocessing profile.
    pub fn new(
        endpoint: &str,
        agent: &str,
        profile: &str,
        mode: TravelMode,
        graph: Provenance,
        interval: Duration,
    ) -> Result<Self> {
        let endpoint_url = url::Url::parse(endpoint).map_err(|_| Error::InvalidQuery)?;
        if endpoint_url.host_str().is_some_and(|host| {
            host.trim_end_matches('.')
                .eq_ignore_ascii_case("router.project-osrm.org")
        }) {
            return Err(Error::Rejected);
        }
        if profile.is_empty()
            || profile.len() > 64
            || !profile
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
        {
            return Err(Error::InvalidQuery);
        }
        Ok(Self {
            http: rom_map_http::Config::new(endpoint, agent, 1048576, interval, 1)?,
            profile: profile.into(),
            mode,
            graph,
            snap_radius: None,
        })
    }
    /// Select the required finite maximum snapping distance in metres for every waypoint.
    pub fn with_snap_radius(mut self, radius: rom_map_core::Metres) -> Result<Self> {
        let radius = radius.as_metres().to_string();
        // The maximum 25-waypoint radii field must fit the shared transport's 4096-byte bound.
        if radius.len().saturating_add(1).saturating_mul(25) > 4096 {
            return Err(Error::TooLarge);
        }
        self.snap_radius = Some(radius);
        Ok(self)
    }
    /// Use a host-approved private certificate authority.
    pub fn with_ca(mut self, ca: Vec<u8>) -> Result<Self> {
        self.http = self.http.with_ca(ca)?;
        Ok(self)
    }
}
/// Optional OSRM routing capability without Resource writes or browser activation.
pub struct Osrm {
    http: rom_map_http::Http,
    profile: String,
    mode: TravelMode,
    graph: Provenance,
    snap_radius: String,
}
impl Osrm {
    /// Prepare the selected adapter without opening connections; reject a missing explicit snap radius.
    pub fn new(config: Config) -> Result<Self> {
        let snap_radius = config.snap_radius.ok_or(Error::InvalidQuery)?;
        Ok(Self {
            http: rom_map_http::Http::new(config.http)?,
            profile: config.profile,
            mode: config.mode,
            graph: config.graph,
            snap_radius,
        })
    }
}
impl Routing for Osrm {
    fn route<'a>(
        &'a self,
        query: &'a RouteQuery,
        context: &'a RequestContext,
    ) -> ProviderFuture<'a, Option<Route>> {
        Box::pin(async move {
            context
                .run(async {
                    if query.travel_mode() != self.mode {
                        return Err(Error::Unsupported);
                    }
                    let coordinates = query
                        .waypoints()
                        .iter()
                        .map(|point| {
                            format!("{},{}", point.longitude_degrees(), point.latitude_degrees())
                        })
                        .collect::<Vec<_>>()
                        .join(";");
                    let radiuses =
                        vec![self.snap_radius.as_str(); query.waypoints().len()].join(";");
                    let response = self
                        .http
                        .get_native(
                            &["route", "v1", &self.profile, &coordinates],
                            &[
                                ("radiuses", radiuses.as_str()),
                                ("geometries", "geojson"),
                                ("overview", "full"),
                                ("alternatives", "false"),
                                ("steps", "false"),
                                ("generate_hints", "false"),
                                ("skip_waypoints", "true"),
                            ],
                            context,
                        )
                        .await?;
                    let credit = self.graph.attribution();
                    let graph = Provenance::new(
                        self.graph.provider(),
                        self.graph.source_id(),
                        rom_map_core::Attribution::new(credit.text(), credit.link())?,
                    )?;
                    crate::route_response(
                        response.body(),
                        response.status() == rom_map_http::ReadStatus::Success,
                        graph,
                    )
                })
                .await
        })
    }
}
