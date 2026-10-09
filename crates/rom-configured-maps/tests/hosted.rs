//! Actual controlled TLS TileJSON protocol, separate from a real tileserver qualification.
#[cfg(feature = "controlled-fixture")]
mod controlled {
    use rom_configured_maps::HostedRaster;
    use rom_map_core::{
        Attribution, BrowserPolicy, Cancellation, Error, RasterTiles, RequestContext,
    };
    use std::{
        io::{BufRead, BufReader},
        process::{Child, Command, Stdio},
        sync::mpsc,
        time::Duration,
    };
    struct Fixture(Child);
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    #[tokio::test(flavor = "current_thread")]
    async fn actual_https_metadata_is_bounded_approved_and_never_discloses_server_key() {
        let mut fixture = Fixture(
            Command::new("node")
                .arg(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/tests/hosted-receiver.mjs"
                ))
                .stdout(Stdio::piped())
                .spawn()
                .unwrap(),
        );
        let out = fixture.0.stdout.take().unwrap();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut line = String::new();
            BufReader::new(out).read_line(&mut line).unwrap();
            let _ = tx.send(line);
        });
        let port = rx
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .trim()
            .parse::<u16>()
            .unwrap();
        let tls = std::env::var("ROM_EXTRAS_MAP_FIXTURE_TLS").unwrap();
        for document in [
            "valid.json",
            "evil.json",
            "limited.json",
            "large.json",
            "slow.json",
        ] {
            let http = rom_map_http::Config::new(
                &format!("https://127.0.0.1:{port}/"),
                "ROM metadata fixture/1",
                1024,
                Duration::ZERO,
                1,
            )
            .unwrap()
            .with_ca(std::fs::read(format!("{tls}/ca.pem")).unwrap())
            .unwrap()
            .with_server_query_key("key", "server-fixture-only".into())
            .unwrap();
            let provider = HostedRaster::new(
                http,
                document,
                BrowserPolicy::new(&["https://tiles.operator.test"]).unwrap(),
                Attribution::new("Authored metadata fixture", None).unwrap(),
            )
            .unwrap();
            let context =
                RequestContext::new(Duration::from_millis(100), Cancellation::new()).unwrap();
            let result = provider.raster_tiles(&context).await;
            match document {
                "valid.json" => {
                    let browser = result.unwrap().source().to_browser_json();
                    assert_eq!(
                        browser["tiles"][0],
                        "https://tiles.operator.test/{z}/{x}/{y}.png"
                    );
                    assert!(!browser.to_string().contains("server-fixture-only"));
                }
                "evil.json" => assert!(result.is_err()),
                "limited.json" => assert!(matches!(result, Err(Error::RateLimited { .. }))),
                "large.json" => assert!(matches!(result, Err(Error::TooLarge))),
                "slow.json" => assert!(matches!(result, Err(Error::Timeout))),
                _ => unreachable!(),
            }
            let cancel = Cancellation::new();
            cancel.cancel();
            let context = RequestContext::new(Duration::from_secs(1), cancel).unwrap();
            assert!(matches!(
                provider.raster_tiles(&context).await,
                Err(Error::Cancelled)
            ));
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn hosted_vector_keeps_native_layers_and_rejects_raster_manifest() {
        let mut fixture = Fixture(
            Command::new("node")
                .arg(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/tests/hosted-receiver.mjs"
                ))
                .stdout(Stdio::piped())
                .spawn()
                .unwrap(),
        );
        let out = fixture.0.stdout.take().unwrap();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut line = String::new();
            BufReader::new(out).read_line(&mut line).unwrap();
            let _ = tx.send(line);
        });
        let port = rx
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .trim()
            .parse::<u16>()
            .unwrap();
        let tls = std::env::var("ROM_EXTRAS_MAP_FIXTURE_TLS").unwrap();
        for document in ["vector.json", "valid.json"] {
            let http = rom_map_http::Config::new(
                &format!("https://127.0.0.1:{port}/"),
                "ROM vector fixture/1",
                1024,
                Duration::ZERO,
                1,
            )
            .unwrap()
            .with_ca(std::fs::read(format!("{tls}/ca.pem")).unwrap())
            .unwrap()
            .with_server_query_key("key", "server-fixture-only".into())
            .unwrap();
            let provider = rom_configured_maps::HostedVector::new(
                http,
                document,
                BrowserPolicy::new(&["https://tiles.operator.test"]).unwrap(),
                Attribution::new("Authored vector fixture", None).unwrap(),
            )
            .unwrap();
            let context = RequestContext::new(Duration::from_secs(1), Cancellation::new()).unwrap();
            use rom_map_core::VectorTiles;
            let result = provider.vector_tiles(&context).await;
            if document == "vector.json" {
                assert_eq!(
                    result.unwrap().source().to_browser_json()["vector_layers"][0]["id"],
                    "Roads/00001"
                );
            } else {
                assert!(result.is_err());
            }
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn hosted_style_retains_identifiers_and_never_fetches_nested_manifest() {
        let mut fixture = Fixture(
            Command::new("node")
                .arg(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/tests/hosted-receiver.mjs"
                ))
                .stdout(Stdio::piped())
                .spawn()
                .unwrap(),
        );
        let out = fixture.0.stdout.take().unwrap();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut line = String::new();
            BufReader::new(out).read_line(&mut line).unwrap();
            let _ = tx.send(line);
        });
        let port = rx
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .trim()
            .parse::<u16>()
            .unwrap();
        let tls = std::env::var("ROM_EXTRAS_MAP_FIXTURE_TLS").unwrap();
        for document in ["style.json", "opaque-style.json"] {
            let http = rom_map_http::Config::new(
                &format!("https://127.0.0.1:{port}/"),
                "ROM style fixture/1",
                1024,
                Duration::ZERO,
                1,
            )
            .unwrap()
            .with_ca(std::fs::read(format!("{tls}/ca.pem")).unwrap())
            .unwrap()
            .with_server_query_key("key", "server-fixture-only".into())
            .unwrap();
            let provider = rom_configured_maps::HostedStyle::new(
                http,
                document,
                BrowserPolicy::new(&["https://tiles.operator.test"]).unwrap(),
                vec![Attribution::new("Authored style fixture", None).unwrap()],
                vec![],
            )
            .unwrap();
            let context = RequestContext::new(Duration::from_secs(1), Cancellation::new()).unwrap();
            use rom_map_core::Styles;
            let result = provider.style(&context).await;
            if document == "style.json" {
                let browser = result.unwrap().to_browser_json();
                assert_eq!(browser["style"]["layers"][0]["id"], "Background/00001");
                assert!(!browser.to_string().contains("server-fixture-only"));
            } else {
                assert!(result.is_err());
            }
        }
    }
}
