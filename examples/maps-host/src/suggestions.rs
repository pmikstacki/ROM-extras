use rom_map_core::{Capabilities, Error, GeocodeQuery, GeocodeResults, RequestContext, Result};
/// The host selects a capability and checks current query authority before and after I/O.
/// Suggestions retain provider provenance and credits; this operation performs no ROM writes.
pub async fn query_suggestions(
    capabilities: &Capabilities<'_>,
    query: &GeocodeQuery,
    context: &RequestContext,
    mut authorize: impl FnMut() -> bool,
) -> Result<GeocodeResults> {
    context
        .run(async {
            if !authorize() {
                return Err(Error::Rejected);
            }
            let outcome = capabilities.geocoding()?.geocode(query, context).await;
            if !authorize() {
                return Err(Error::Rejected);
            }
            outcome
        })
        .await
}
