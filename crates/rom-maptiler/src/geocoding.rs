//! Explicit forward and reverse requests; no IP proximity, retries or cache.
use crate::{MapTiler, geocode_response};
use rom_map_core::{
    Coordinate, Error, GeocodeQuery, GeocodeResults, Geocoding, ProviderFuture, RequestContext,
    ReverseGeocoding,
};
impl Geocoding for MapTiler {
    fn geocode<'a>(
        &'a self,
        query: &'a GeocodeQuery,
        context: &'a RequestContext,
    ) -> ProviderFuture<'a, GeocodeResults> {
        Box::pin(async move {
            context
                .run(async {
                    if query.limit() > 10 {
                        return Err(Error::InvalidQuery);
                    }
                    // The native endpoint also dispatches comma coordinates and semicolon batches.
                    // Do not turn a text capability into either operation implicitly.
                    let components: Vec<_> = query.text().split(',').collect();
                    if query.text().contains(';')
                        || (components.len() == 2
                            && components
                                .iter()
                                .all(|part| part.trim().parse::<f64>().is_ok()))
                    {
                        return Err(Error::InvalidQuery);
                    }
                    let path = format!("{}.json", query.text());
                    let limit = query.limit().to_string();
                    let mut params = vec![("limit", limit.as_str()), ("autocomplete", "false")];
                    let bbox;
                    if let Some(bounds) = query.bounds() {
                        if bounds.crosses_antimeridian() {
                            return Err(Error::Unsupported);
                        }
                        let [west, south, east, north] = bounds.west_south_east_north();
                        bbox = format!("{west},{south},{east},{north}");
                        params.push(("bbox", bbox.as_str()));
                    }
                    let bytes = self
                        .http
                        .get(&["geocoding", &path], &params, context)
                        .await?;
                    geocode_response(&bytes, query.limit(), &self.provider, &self.attribution)
                })
                .await
        })
    }
}
impl ReverseGeocoding for MapTiler {
    fn reverse<'a>(
        &'a self,
        coordinate: Coordinate,
        context: &'a RequestContext,
    ) -> ProviderFuture<'a, GeocodeResults> {
        Box::pin(async move {
            context
                .run(async {
                    let path = format!(
                        "{},{}.json",
                        coordinate.longitude_degrees(),
                        coordinate.latitude_degrees()
                    );
                    let bytes = self
                        .http
                        .get(&["geocoding", &path], &[("limit", "1")], context)
                        .await?;
                    geocode_response(&bytes, 1, &self.provider, &self.attribution)
                })
                .await
        })
    }
}
