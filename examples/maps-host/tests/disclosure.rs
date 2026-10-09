use rom::{Key, ProjectedView, json};
use rom_maps_host_example::projected_point;
fn key() -> Key {
    Key {
        kind: "Places".into(),
        id: "ę/0001:Straße".into(),
    }
}
fn view(value: rom::Value) -> ProjectedView {
    ProjectedView {
        key: key(),
        revision: 3,
        value: value.as_object().cloned(),
    }
}
#[test]
fn preserves_identity_and_whitelists_fields() {
    let p = projected_point(
        &key(),
        view(json!({"title":"Place","longitude":180,"latitude":-90,"secret":"hidden"})),
    )
    .unwrap()
    .unwrap();
    assert_eq!(p.location.key(), &key());
    assert_eq!(p.location.coordinate().longitude_latitude(), [180.0, -90.0]);
    assert_eq!(p.title, "Place");
}
#[test]
fn hidden_missing_null_invalid_and_tombstone_do_not_become_points() {
    for value in [
        json!({"title":"Hidden","latitude":52}),
        json!({"title":"Null","longitude":null,"latitude":52}),
        json!({"title":"Invalid","longitude":181,"latitude":52}),
        json!({"title":"String","longitude":"21","latitude":52}),
        json!(null),
    ] {
        assert!(projected_point(&key(), view(value)).unwrap().is_none());
    }
}
#[test]
fn mismatched_key_is_rejected() {
    let mut v = view(json!({"title":"Place","longitude":21,"latitude":52}));
    v.key.kind = "Other".into();
    assert!(projected_point(&key(), v).is_err());
}
