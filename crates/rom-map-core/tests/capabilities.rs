//! Local contract tests; these are not deployed geocoder or routing qualification.
use rom_map_core::{
    Cancellation, Capabilities, Coordinate, Error, GeocodeQuery, GeocodeResults, Geocoding,
    ProviderFuture, RequestContext, RouteQuery, TravelMode,
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
struct EmptyGeocoder;
impl Geocoding for EmptyGeocoder {
    fn geocode<'a>(
        &'a self,
        query: &'a GeocodeQuery,
        context: &'a RequestContext,
    ) -> ProviderFuture<'a, GeocodeResults> {
        Box::pin(async move {
            context
                .run(async { GeocodeResults::new(vec![], query.limit()) })
                .await
        })
    }
}
#[tokio::test(flavor = "current_thread")]
async fn missing_capabilities_fail_explicitly_and_present_geocoder_does_not_require_routing() {
    let absent = Capabilities::new();
    assert!(matches!(absent.geocoding(), Err(Error::Unsupported)));
    assert!(matches!(
        absent.reverse_geocoding(),
        Err(Error::Unsupported)
    ));
    assert!(matches!(absent.routing(), Err(Error::Unsupported)));
    assert!(matches!(absent.styles(), Err(Error::Unsupported)));
    assert!(matches!(absent.raster_tiles(), Err(Error::Unsupported)));
    assert!(matches!(absent.vector_tiles(), Err(Error::Unsupported)));
    let provider = EmptyGeocoder;
    let selected = Capabilities::new().with_geocoding(&provider);
    let context = RequestContext::new(Duration::from_secs(5), Cancellation::new()).unwrap();
    let query = GeocodeQuery::new("explicit host query", 1).unwrap();
    assert!(
        selected
            .geocoding()
            .unwrap()
            .geocode(&query, &context)
            .await
            .unwrap()
            .results()
            .is_empty()
    );
    assert!(matches!(selected.routing(), Err(Error::Unsupported)));
}
#[tokio::test(flavor = "current_thread")]
async fn cancellation_before_dispatch_never_polls_the_operation() {
    let token = Cancellation::new();
    token.cancel();
    let context = RequestContext::new(Duration::from_secs(5), token).unwrap();
    let polls = AtomicUsize::new(0);
    let result = context
        .run(async {
            polls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
        .await;
    assert_eq!(result, Err(Error::Cancelled));
    assert_eq!(polls.load(Ordering::SeqCst), 0);
}
#[tokio::test(flavor = "current_thread")]
async fn cancellation_interrupts_a_pending_future_and_deadlines_are_finite() {
    let token = Cancellation::new();
    let context = RequestContext::new(Duration::from_secs(5), token.clone()).unwrap();
    let entered = Arc::new(AtomicUsize::new(0));
    let observe = entered.clone();
    let operation = async move {
        observe.fetch_add(1, Ordering::SeqCst);
        std::future::pending::<rom_map_core::Result<()>>().await
    };
    let cancel = async {
        while entered.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }
        token.cancel();
    };
    let (result, ()) = tokio::time::timeout(Duration::from_secs(1), async {
        tokio::join!(context.run(operation), cancel)
    })
    .await
    .expect("bounded cancellation fixture");
    assert_eq!(result, Err(Error::Cancelled));
    let context = RequestContext::new(Duration::from_millis(10), Cancellation::new()).unwrap();
    assert_eq!(
        context
            .run(std::future::pending::<rom_map_core::Result<()>>())
            .await,
        Err(Error::Timeout)
    );
    assert!(matches!(
        RequestContext::new(Duration::ZERO, Cancellation::new()),
        Err(Error::InvalidQuery)
    ));
    assert!(matches!(
        RequestContext::new(Duration::from_secs(61), Cancellation::new()),
        Err(Error::InvalidQuery)
    ));
}
#[test]
fn route_waypoints_are_bounded_and_preserve_the_requested_mode_and_order() {
    let a = Coordinate::new(19.0, 50.0).unwrap();
    let b = Coordinate::new(20.0, 51.0).unwrap();
    let q = RouteQuery::new(vec![a, b], TravelMode::Walking).unwrap();
    assert_eq!(q.waypoints(), &[a, b]);
    assert_eq!(q.travel_mode(), TravelMode::Walking);
    assert!(matches!(
        RouteQuery::new(vec![a], TravelMode::Driving),
        Err(Error::InvalidQuery)
    ));
    assert!(matches!(
        RouteQuery::new(vec![a; 26], TravelMode::Driving),
        Err(Error::TooLarge)
    ));
}
