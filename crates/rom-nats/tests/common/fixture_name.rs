//! Shared selection of labelled owned NATS fixtures.
pub fn valid_name(name: &str) -> bool {
    let prefix = "rom-extras-nats-";
    name.starts_with(prefix)
        && name.len() > prefix.len()
        && name.len() <= 128
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'-' | b'_'))
}
pub fn selected(tls: bool) -> String {
    let (variable, default, label) = if tls {
        (
            "ROM_EXTRAS_NATS_TLS_CONTAINER",
            "rom-extras-nats-tls-20261008",
            "nats-tls",
        )
    } else {
        (
            "ROM_EXTRAS_NATS_CONTAINER",
            "rom-extras-nats-20261008",
            "nats",
        )
    };
    let name = match std::env::var(variable) {
        Ok(value) => value,
        Err(std::env::VarError::NotPresent) => default.into(),
        Err(_) => panic!("invalid fixture selector encoding"),
    };
    assert!(valid_name(&name), "invalid owned NATS fixture name");
    let result = std::process::Command::new("docker")
        .args([
            "inspect",
            &name,
            "--format",
            "{{index .Config.Labels \"rom-extras.fixture\"}}",
        ])
        .output()
        .expect("fixture inspection failed");
    assert!(result.status.success(), "fixture inspection failed");
    assert_eq!(
        String::from_utf8(result.stdout)
            .expect("fixture label encoding")
            .trim(),
        label,
        "fixture ownership label mismatch"
    );
    name
}
pub fn endpoint_matches(tls: bool, binding: &str, url: &str) -> bool {
    let Some(port) = binding
        .strip_prefix("127.0.0.1:")
        .and_then(|p| p.parse::<u16>().ok())
        .filter(|p| *p > 0)
    else {
        return false;
    };
    let scheme = if tls { "tls" } else { "nats" };
    url == format!("{scheme}://127.0.0.1:{port}")
}
pub fn assert_endpoint(name: &str, tls: bool, url: &str) {
    assert!(valid_name(name), "invalid owned fixture name");
    let result = std::process::Command::new("docker")
        .args(["port", name, "4222/tcp"])
        .output()
        .expect("fixture binding inspection failed");
    assert!(result.status.success(), "fixture binding inspection failed");
    let binding = String::from_utf8(result.stdout).expect("fixture binding encoding");
    assert!(
        endpoint_matches(tls, binding.trim(), url),
        "fixture endpoint must match owned loopback binding"
    );
}
