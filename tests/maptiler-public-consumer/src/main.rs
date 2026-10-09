use rom_map_core::{
    Accuracy, Attribution, Cancellation, Coordinate, GeocodeQuery, Geocoding, RequestContext,
    ReverseGeocoding,
};
use rom_maptiler::{AttributionProfile, Config, MapTiler};
use std::time::Duration;
mod maps;
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert_eq!(
        args.len(),
        2,
        "explicit controlled endpoint and CA path required"
    );
    let profile =
        AttributionProfile::new("fixture credit", Attribution::new("Fixture", None).unwrap())
            .unwrap();
    let adapter = MapTiler::new(
        Config::new(
            &args[0],
            "IndependentConsumer/1",
            "fixture",
            Duration::ZERO,
            profile,
        )
        .unwrap()
        .with_server_key("synthetic-secret".into())
        .unwrap()
        .with_ca(std::fs::read(&args[1]).unwrap())
        .unwrap(),
    )
    .unwrap();
    let context = RequestContext::new(Duration::from_secs(2), Cancellation::new()).unwrap();
    let found = adapter
        .geocode(&GeocodeQuery::new("Warsaw & test", 1).unwrap(), &context)
        .await
        .unwrap();
    assert_eq!(
        found.results()[0].provenance().source_id(),
        "municipality.00001"
    );
    assert_eq!(
        found.results()[0].coordinate().longitude_latitude(),
        [21.0, 52.0]
    );
    assert!(matches!(found.results()[0].accuracy(), Accuracy::Unknown));
    // The fixture host explicitly supplies its own Resource key after approval.
    // Native geocoder identity remains provenance and is never the Resource key.
    let approved_key = rom::Key {
        kind: "Places".into(),
        id: "ę/0001:Straße".into(),
    };
    let location =
        rom_map_core::ResourceLocation::new(approved_key.clone(), found.results()[0].coordinate());
    assert_eq!(location.key(), &approved_key);
    assert_ne!(
        location.key().id,
        found.results()[0].provenance().source_id()
    );
    let reverse = adapter
        .reverse(Coordinate::new(21.0, 52.0).unwrap(), &context)
        .await
        .unwrap();
    assert_eq!(
        reverse.results()[0].provenance().source_id(),
        "municipality.00001"
    );
    assert!(
        adapter
            .reverse(Coordinate::new(0.0, 0.0).unwrap(), &context)
            .await
            .unwrap()
            .results()
            .is_empty()
    );
    maps::run(&args[0], &args[1]).await;
    println!("Independent public API consumer: controlled HTTPS forward/reverse passed");
}
