//! Spatial and host-identity behavior through exported contracts only.
use rom_map_core::{
    BoundingBox, Cancellation, Capabilities, Coordinate, Error, Metres, RequestContext,
    ResourceLocation, RouteGeometry, Seconds,
};
use std::time::Duration;
pub async fn run() {
    let coordinate = Coordinate::new(19.94, 50.06).unwrap();
    let key = rom::Key {
        kind: "Places".into(),
        id: "ę/0001:Straße".into(),
    };
    let location = ResourceLocation::new(key.clone(), coordinate);
    assert_eq!(location.key(), &key);
    let disclosed = serde_json::json!({"key":location.key(),"coordinates":location.coordinate().longitude_latitude()});
    assert_eq!(disclosed["key"]["id"], "ę/0001:Straße");
    assert_eq!(disclosed["coordinates"], serde_json::json!([19.94, 50.06]));
    let bounds = BoundingBox::new(177.0, -20.0, -178.0, -16.0).unwrap();
    assert!(bounds.contains(Coordinate::new(-179.0, -18.0).unwrap()));
    assert!(!bounds.contains(coordinate));
    let geometry =
        RouteGeometry::line_string(vec![coordinate, Coordinate::new(20.0, 50.1).unwrap()]).unwrap();
    assert_eq!(geometry.to_geojson()["type"], "LineString");
    assert_eq!(Metres::new(1500.0).unwrap().as_metres(), 1500.0);
    assert_eq!(Seconds::new(60.0).unwrap().as_seconds(), 60.0);
    assert!(matches!(
        Capabilities::new().routing(),
        Err(Error::Unsupported)
    ));
    let token = Cancellation::new();
    token.cancel();
    let context = RequestContext::new(Duration::from_secs(5), token).unwrap();
    assert_eq!(context.run(async { Ok(()) }).await, Err(Error::Cancelled));
    println!(
        "Public map spatial/capability/cancellation contracts: passed; no provider service or UI claim"
    );
}
