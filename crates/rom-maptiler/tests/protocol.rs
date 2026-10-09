//! Actual controlled HTTPS MapTiler-shaped exchange, not native service qualification.
#[cfg(feature = "controlled-fixture")]
mod controlled {
    use rom_map_core::{
        Cancellation, Coordinate, Error, GeocodeQuery, Geocoding, RequestContext, ReverseGeocoding,
    };
    use rom_maptiler::{AttributionProfile, Config, MapTiler};
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
    fn fixture() -> (Fixture, MapTiler) {
        let mut fixture = Fixture(
            Command::new("node")
                .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/receiver.mjs"))
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let out = fixture.0.stdout.take().unwrap();
        let (send, receive) = mpsc::channel();
        std::thread::spawn(move || {
            let mut port = String::new();
            let result = BufReader::new(out).read_line(&mut port);
            let _ = send.send((result, port));
        });
        let (result, port) = receive.recv_timeout(Duration::from_secs(5)).unwrap();
        result.unwrap();
        let port: u16 = port.trim().parse().unwrap();
        let config = Config::new(
            &format!("https://127.0.0.1:{port}/operator/"),
            "Host/1 (operator@example.test)",
            "private-maptiler",
            Duration::ZERO,
            AttributionProfile::new(
                "fixture credit",
                rom_map_core::Attribution::new("Fixture", None).unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
        .with_server_key("synthetic-secret".into())
        .unwrap()
        .with_ca(
            std::fs::read(
                std::path::Path::new(
                    &std::env::var("ROM_EXTRAS_MAP_FIXTURE_TLS")
                        .expect("controlled fixture TLS directory required"),
                )
                .join("ca.pem"),
            )
            .unwrap(),
        )
        .unwrap();
        (fixture, MapTiler::new(config).unwrap())
    }
    #[tokio::test(flavor = "current_thread")]
    async fn actual_https_search_preserves_native_result_and_encodes_query() {
        let (_fixture, adapter) = fixture();
        let context = RequestContext::new(Duration::from_secs(2), Cancellation::new()).unwrap();
        let result = adapter
            .geocode(&GeocodeQuery::new("Warsaw & test", 1).unwrap(), &context)
            .await
            .unwrap();
        assert_eq!(
            result.results()[0].coordinate().longitude_latitude(),
            [21.0, 52.0]
        );
        assert_eq!(
            result.results()[0].provenance().source_id(),
            "municipality.00001"
        );
        let bounded = GeocodeQuery::new("bounded", 1)
            .unwrap()
            .with_bounds(rom_map_core::BoundingBox::new(20.0, 51.0, 22.0, 53.0).unwrap());
        assert_eq!(
            adapter
                .geocode(&bounded, &context)
                .await
                .unwrap()
                .results()
                .len(),
            1
        );
        let reverse = adapter
            .reverse(Coordinate::new(21.0, 52.0).unwrap(), &context)
            .await
            .unwrap();
        assert_eq!(
            reverse.results()[0].provenance().source_id(),
            "municipality.00001"
        );
        let empty = adapter
            .reverse(Coordinate::new(0.0, 0.0).unwrap(), &context)
            .await
            .unwrap();
        assert!(empty.results().is_empty());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn adapter_bounds_errors_empty_and_deadline_remain_safe() {
        let (_fixture, adapter) = fixture();
        let context = RequestContext::new(Duration::from_secs(2), Cancellation::new()).unwrap();
        let empty = adapter
            .geocode(&GeocodeQuery::new("empty", 1).unwrap(), &context)
            .await
            .unwrap();
        assert!(empty.results().is_empty());
        for (query, expected) in [
            ("excess", Error::TooLarge),
            ("invalid", Error::InvalidResponse),
            ("huge", Error::TooLarge),
            ("backend", Error::Unavailable),
            ("redirect", Error::Rejected),
            (
                "rate",
                Error::RateLimited {
                    retry_after_seconds: Some(2),
                },
            ),
        ] {
            let error = match adapter
                .geocode(&GeocodeQuery::new(query, 1).unwrap(), &context)
                .await
            {
                Ok(_) => panic!("unexpected approved result"),
                Err(error) => error,
            };
            assert_eq!(error, expected);
            assert!(!error.to_string().contains("synthetic-secret"));
        }
        let short = RequestContext::new(Duration::from_millis(40), Cancellation::new()).unwrap();
        assert!(matches!(
            adapter
                .geocode(&GeocodeQuery::new("slow", 1).unwrap(), &short)
                .await,
            Err(Error::Timeout)
        ));
        let cancel = Cancellation::new();
        cancel.cancel();
        let cancelled = RequestContext::new(Duration::from_secs(1), cancel).unwrap();
        assert!(matches!(
            adapter
                .geocode(&GeocodeQuery::new("Warsaw & test", 1).unwrap(), &cancelled)
                .await,
            Err(Error::Cancelled)
        ));
        let cancel = Cancellation::new();
        let in_flight = RequestContext::new(Duration::from_secs(2), cancel.clone()).unwrap();
        let query = GeocodeQuery::new("slow", 1).unwrap();
        let (result, ()) = tokio::join!(adapter.geocode(&query, &in_flight), async {
            tokio::time::sleep(Duration::from_millis(30)).await;
            cancel.cancel();
        });
        assert!(matches!(result, Err(Error::Cancelled)));
        // Cancellation/deadline release admission for the next approved request.
        assert_eq!(
            adapter
                .geocode(&GeocodeQuery::new("Warsaw & test", 1).unwrap(), &context)
                .await
                .unwrap()
                .results()
                .len(),
            1
        );
    }
}
