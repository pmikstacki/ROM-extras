//! Fixture selector names are bounded and cannot become arbitrary Docker arguments.
#[path = "common/fixture_name.rs"]
mod fixture_name;
#[test]
fn only_owned_ascii_fixture_names_are_admitted() {
    assert!(fixture_name::valid_name("rom-extras-nats-20261008"));
    assert!(fixture_name::valid_name(
        "rom-extras-nats-qualification-20261009"
    ));
    for invalid in [
        "production",
        "--help",
        "rom-extras-nats-",
        "rom-extras-nats-x y",
        "rom-extras-nats-é",
        "rom-extras-nats-$(id)",
    ] {
        assert!(!fixture_name::valid_name(invalid));
    }
    assert!(!fixture_name::valid_name(&format!(
        "rom-extras-nats-{}",
        "a".repeat(128)
    )));
}

const _: fn(bool) -> String = fixture_name::selected;
const _: fn(&str, bool, &str) = fixture_name::assert_endpoint;
#[test]
fn endpoint_must_match_loopback_binding_and_transport() {
    assert!(fixture_name::endpoint_matches(
        false,
        "127.0.0.1:55465",
        "nats://127.0.0.1:55465"
    ));
    for (binding, url) in [
        ("0.0.0.0:55465", "nats://127.0.0.1:55465"),
        ("127.0.0.1:55465", "nats://127.0.0.1:55441"),
        ("127.0.0.1:0", "nats://127.0.0.1:0"),
        ("127.0.0.1:55465", "nats://user:secret@127.0.0.1:55465"),
    ] {
        assert!(!fixture_name::endpoint_matches(false, binding, url));
    }
    assert!(!fixture_name::endpoint_matches(
        true,
        "127.0.0.1:55465",
        "nats://127.0.0.1:55465"
    ));
}
