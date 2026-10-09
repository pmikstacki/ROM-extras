use rom_map_core::{Cancellation, Capabilities, Coordinate, GeocodeQuery, RequestContext};
use rom_nominatim::{Config, Nominatim};
use std::time::Duration;
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut args = std::env::args().skip(1);
    let endpoint = args.next().expect("explicit controlled endpoint required");
    let ca = args.next().expect("explicit controlled CA file required");
    assert!(args.next().is_none());
    let provider = Nominatim::new(
        Config::new(
            &endpoint,
            "Host/1 (operator@example.test)",
            "independent-host",
            Duration::ZERO,
        )
        .unwrap()
        .with_ca(std::fs::read(ca).unwrap())
        .unwrap(),
    )
    .unwrap();
    let capabilities = Capabilities::new()
        .with_geocoding(&provider)
        .with_reverse_geocoding(&provider);
    assert!(capabilities.routing().is_err());
    assert!(capabilities.styles().is_err());
    let context = RequestContext::new(Duration::from_secs(2), Cancellation::new()).unwrap();
    let result = capabilities
        .geocoding()
        .unwrap()
        .geocode(&GeocodeQuery::new("Warsaw & test", 1).unwrap(), &context)
        .await
        .unwrap();
    assert_eq!(
        result.results()[0].provenance().source_id(),
        "relation/00123"
    );
    assert_eq!(
        result.results()[0].provenance().provider(),
        "independent-host"
    );
    let reverse = capabilities
        .reverse_geocoding()
        .unwrap()
        .reverse(Coordinate::new(21.0, 52.0).unwrap(), &context)
        .await
        .unwrap();
    assert_eq!(
        reverse.results()[0].coordinate().longitude_latitude(),
        [21.0, 52.0]
    );
    let empty = capabilities
        .reverse_geocoding()
        .unwrap()
        .reverse(Coordinate::new(0.0, 0.0).unwrap(), &context)
        .await
        .unwrap();
    assert!(empty.results().is_empty());
    println!(
        "Independent public map adapter consumer: search/reverse/no-coverage/missing capabilities passed"
    );
}
