//! Actual controlled HTTPS Nominatim-shaped exchange, not native service qualification.
#[cfg(feature = "controlled-fixture")]
mod controlled {
    use rom_map_core::{
        Cancellation, Coordinate, Error, GeocodeQuery, Geocoding, RequestContext, ReverseGeocoding,
    };
    use rom_nominatim::{Config, Nominatim};
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
    fn fixture() -> (Fixture, Nominatim) {
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
            "private-nominatim",
            Duration::ZERO,
        )
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
        (fixture, Nominatim::new(config).unwrap())
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
            "relation/00123"
        );
        let reverse = adapter
            .reverse(Coordinate::new(21.0, 52.0).unwrap(), &context)
            .await
            .unwrap();
        assert_eq!(
            reverse.results()[0].provenance().source_id(),
            "relation/00123"
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
