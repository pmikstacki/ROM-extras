//! Actual authored OSRM-shaped HTTPS exchange, not native OSRM qualification.
#[cfg(feature = "controlled-fixture")]
mod controlled {
    use rom_map_core::{
        Attribution, Cancellation, Coordinate, Error, Provenance, RequestContext, RouteQuery,
        Routing, TravelMode,
    };
    use rom_osrm::{Config, Osrm};
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
    fn graph() -> Provenance {
        Provenance::new(
            "private-osrm",
            "operator-region.osrm",
            Attribution::new("© OpenStreetMap contributors", None).unwrap(),
        )
        .unwrap()
    }
    fn query(first: [f64; 2], mode: TravelMode) -> RouteQuery {
        RouteQuery::new(
            vec![
                Coordinate::new(first[0], first[1]).unwrap(),
                Coordinate::new(21.1, 52.1).unwrap(),
            ],
            mode,
        )
        .unwrap()
    }
    #[tokio::test(flavor = "current_thread")]
    async fn actual_https_route_profile_absence_and_errors_preserve_contract() {
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
        let endpoint = format!("https://127.0.0.1:{port}/operator/");
        let ca = std::fs::read(
            std::path::Path::new(&std::env::var("ROM_EXTRAS_MAP_FIXTURE_TLS").unwrap())
                .join("ca.pem"),
        )
        .unwrap();
        let adapter = Osrm::new(
            Config::new(
                &endpoint,
                "Host/1",
                "car",
                TravelMode::Driving,
                graph(),
                Duration::ZERO,
            )
            .unwrap()
            .with_snap_radius(rom_map_core::Metres::new(5.0).unwrap())
            .unwrap()
            .with_ca(ca.clone())
            .unwrap(),
        )
        .unwrap();
        let context = RequestContext::new(Duration::from_secs(2), Cancellation::new()).unwrap();
        let route = adapter
            .route(&query([21.0, 52.0], TravelMode::Driving), &context)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(route.distance().as_metres(), 900.0);
        assert_eq!(route.duration().as_seconds(), 60.0);
        assert_eq!(route.provenance().source_id(), "operator-region.osrm");
        assert!(matches!(
            adapter
                .route(&query([21.0, 52.0], TravelMode::Walking), &context)
                .await,
            Err(Error::Unsupported)
        ));
        let reader = rom_map_http::Http::new(
            rom_map_http::Config::new(&endpoint, "Host/1", 1024, Duration::ZERO, 1)
                .unwrap()
                .with_ca(ca)
                .unwrap(),
        )
        .unwrap();
        let state: serde_json::Value =
            serde_json::from_slice(&reader.get(&["events"], &[], &context).await.unwrap()).unwrap();
        assert_eq!(state["requests"], 1);
        for coordinate in [[0.0, 0.0], [180.0, 90.0]] {
            assert!(
                adapter
                    .route(&query(coordinate, TravelMode::Driving), &context)
                    .await
                    .unwrap()
                    .is_none()
            );
        }
        let error = match adapter
            .route(&query([-1.0, 52.0], TravelMode::Driving), &context)
            .await
        {
            Ok(_) => panic!("invalid query approved"),
            Err(e) => e,
        };
        assert_eq!(error, Error::Rejected);
        assert!(!error.to_string().contains("synthetic-secret"));
    }
}
