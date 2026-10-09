use rom_map_core::{
    Cancellation, Capabilities, Error, GeocodeQuery, GeocodeResults, Geocoding, ProviderFuture,
    RequestContext,
};
use rom_maps_host_example::query_suggestions;
use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
struct Fixture(AtomicUsize);
impl Geocoding for Fixture {
    fn geocode<'a>(
        &'a self,
        _: &'a GeocodeQuery,
        _: &'a RequestContext,
    ) -> ProviderFuture<'a, GeocodeResults> {
        Box::pin(async move {
            self.0.fetch_add(1, Ordering::SeqCst);
            GeocodeResults::new(vec![], 1)
        })
    }
}
#[tokio::test]
async fn host_rejection_before_and_after_dispatch() {
    let provider = Fixture(AtomicUsize::new(0));
    let caps = Capabilities::new().with_geocoding(&provider);
    let query = GeocodeQuery::new("Place", 1).unwrap();
    let ctx = RequestContext::new(Duration::from_secs(1), Cancellation::new()).unwrap();
    assert!(matches!(
        query_suggestions(&caps, &query, &ctx, || false).await,
        Err(Error::Rejected)
    ));
    assert_eq!(provider.0.load(Ordering::SeqCst), 0);
    let mut calls = 0;
    assert!(matches!(
        query_suggestions(&caps, &query, &ctx, || {
            calls += 1;
            calls == 1
        })
        .await,
        Err(Error::Rejected)
    ));
    assert_eq!(provider.0.load(Ordering::SeqCst), 1);
    assert!(
        query_suggestions(&caps, &query, &ctx, || true)
            .await
            .unwrap()
            .results()
            .is_empty()
    );
    assert!(matches!(
        query_suggestions(&Capabilities::new(), &query, &ctx, || true).await,
        Err(Error::Unsupported)
    ));
}
