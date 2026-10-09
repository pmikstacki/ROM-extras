//! Actual controlled HTTPS protocol; not native provider/service qualification.
#[cfg(feature = "controlled-fixture")]
mod controlled {
    use rom_map_core::{Cancellation, Error, RequestContext};
    use rom_map_http::{Config, Http};
    use serde_json::Value;
    use std::{
        io::{BufRead, BufReader},
        process::{Child, Command, Stdio},
        sync::mpsc,
        time::Duration,
    };
    struct Fixture {
        child: Child,
        endpoint: String,
    }
    impl Fixture {
        fn start() -> Self {
            let mut child = Command::new("node")
                .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/receiver.mjs"))
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .unwrap();
            let out = child.stdout.take().unwrap();
            let (send, receive) = mpsc::channel();
            std::thread::spawn(move || {
                let mut port = String::new();
                let result = BufReader::new(out).read_line(&mut port);
                let _ = send.send((result, port));
            });
            let mut fixture = Self {
                child,
                endpoint: String::new(),
            };
            let (result, port) = receive.recv_timeout(Duration::from_secs(5)).unwrap();
            result.unwrap();
            let port: u16 = port.trim().parse().unwrap();
            fixture.endpoint = format!("https://127.0.0.1:{port}/operator/");
            fixture
        }
        fn config(&self, limit: usize) -> Config {
            Config::new(
                &self.endpoint,
                "ROM fixture/1 (operator@example.test)",
                limit,
                Duration::ZERO,
                1,
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
            .unwrap()
        }
        fn http(&self) -> Http {
            Http::new(self.config(1024)).unwrap()
        }
        async fn events(&self) -> Value {
            serde_json::from_slice(
                &self
                    .http()
                    .get(&["events"], &[], &context(1000))
                    .await
                    .unwrap(),
            )
            .unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
    fn context(ms: u64) -> RequestContext {
        RequestContext::new(Duration::from_millis(ms), Cancellation::new()).unwrap()
    }

    #[tokio::test(flavor = "current_thread")]
    async fn verified_service_get_is_identified_and_backend_key_stays_out_of_results() {
        let fixture = Fixture::start();
        let http = Http::new(
            fixture
                .config(1024)
                .with_server_query_key("key", "synthetic-backend-secret".into())
                .unwrap(),
        )
        .unwrap();
        let bytes = http.get(&["ok"], &[], &context(1000)).await.unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&bytes).unwrap(),
            serde_json::json!([{"fixture":true}])
        );
        assert!(
            !String::from_utf8(bytes)
                .unwrap()
                .contains("synthetic-backend-secret")
        );
        let state = fixture.events().await;
        assert_eq!(state["requests"], 1);
        assert_eq!(state["identified"], true);
        assert_eq!(state["backendKey"], true);
    }
    #[tokio::test(flavor = "current_thread")]
    async fn rate_rejection_has_bounded_hint_without_retry_or_body_disclosure() {
        let fixture = Fixture::start();
        let error = fixture
            .http()
            .get(&["rate"], &[], &context(1000))
            .await
            .unwrap_err();
        assert_eq!(
            error,
            Error::RateLimited {
                retry_after_seconds: Some(2)
            }
        );
        assert!(!format!("{error:?} {error}").contains("PRIVATE"));
        assert_eq!(fixture.events().await["requests"], 1);
    }
    #[tokio::test(flavor = "current_thread")]
    async fn redirects_are_not_followed_and_response_limits_cover_declared_and_streamed_bytes() {
        let fixture = Fixture::start();
        let http = fixture.http();
        assert_eq!(
            http.get(&["redirect"], &[], &context(1000))
                .await
                .unwrap_err(),
            Error::Rejected
        );
        assert_eq!(fixture.events().await["target"], 0);
        for path in ["declared", "stream"] {
            assert_eq!(
                http.get(&[path], &[], &context(1000)).await.unwrap_err(),
                Error::TooLarge
            );
        }
    }
    #[tokio::test(flavor = "current_thread")]
    async fn timeout_and_inflight_cancellation_release_admission_and_disconnect() {
        let fixture = Fixture::start();
        let http = fixture.http();
        assert_eq!(
            http.get(&["slow"], &[], &context(30)).await.unwrap_err(),
            Error::Timeout
        );
        let token = Cancellation::new();
        let ctx = RequestContext::new(Duration::from_secs(1), token.clone()).unwrap();
        let cancel = async {
            tokio::time::sleep(Duration::from_millis(30)).await;
            token.cancel();
        };
        let (result, ()) = tokio::join!(http.get(&["slow"], &[], &ctx), cancel);
        assert_eq!(result.unwrap_err(), Error::Cancelled);
        assert!(http.get(&["ok"], &[], &context(1000)).await.is_ok());
        assert!(fixture.events().await["aborted"].as_u64().unwrap() >= 1);
    }
    #[tokio::test(flavor = "current_thread")]
    async fn precancelled_request_makes_zero_calls_and_wrong_ca_does_not_connect() {
        let fixture = Fixture::start();
        let token = Cancellation::new();
        token.cancel();
        let ctx = RequestContext::new(Duration::from_secs(1), token).unwrap();
        assert_eq!(
            fixture.http().get(&["ok"], &[], &ctx).await.unwrap_err(),
            Error::Cancelled
        );
        let config = Config::new(
            &fixture.endpoint,
            "ROM fixture/1 (operator@example.test)",
            1024,
            Duration::ZERO,
            1,
        )
        .unwrap();
        assert!(
            Http::new(config)
                .unwrap()
                .get(&["ok"], &[], &context(1000))
                .await
                .is_err()
        );
        assert_eq!(fixture.events().await["requests"], 0);
    }
    #[tokio::test(flavor = "current_thread")]
    async fn occupied_admission_rejects_without_queue_or_extra_request() {
        let fixture = Fixture::start();
        let http = fixture.http();
        let first_context = context(1000);
        let second = async {
            tokio::time::sleep(Duration::from_millis(30)).await;
            http.get(&["ok"], &[], &context(1000)).await
        };
        let (first, rejected) = tokio::join!(http.get(&["slow"], &[], &first_context), second);
        assert!(first.is_ok());
        assert_eq!(rejected.unwrap_err(), Error::Unavailable);
        assert_eq!(fixture.events().await["requests"], 1);
        assert!(http.get(&["ok"], &[], &context(1000)).await.is_ok());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn rate_wait_shares_total_deadline_and_cancelled_wait_does_not_dispatch() {
        let fixture = Fixture::start();
        let config = Config::new(
            &fixture.endpoint,
            "ROM fixture/1 (operator@example.test)",
            1024,
            Duration::from_millis(200),
            1,
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
        let http = Http::new(config).unwrap();
        let first_attempt = std::time::Instant::now();
        assert!(http.get(&["ok"], &[], &context(1000)).await.is_ok());
        assert_eq!(
            http.get(&["ok"], &[], &context(20)).await.unwrap_err(),
            Error::Timeout
        );
        let token = Cancellation::new();
        let ctx = RequestContext::new(Duration::from_secs(1), token.clone()).unwrap();
        let cancel = async {
            tokio::time::sleep(Duration::from_millis(20)).await;
            token.cancel();
        };
        let (cancelled, ()) = tokio::join!(http.get(&["ok"], &[], &ctx), cancel);
        assert_eq!(cancelled.unwrap_err(), Error::Cancelled);
        assert_eq!(fixture.events().await["requests"], 1);
        assert!(http.get(&["ok"], &[], &context(1000)).await.is_ok());
        let events = fixture.events().await;
        assert_eq!(events["requests"], 2);
        // The configured interval bounds client dispatch attempts. Receiver arrival
        // includes a cold TLS handshake for the first call and a reused connection
        // for the second, so receiver timestamps cannot establish that interval.
        assert!(first_attempt.elapsed() >= Duration::from_millis(190));
    }
    #[tokio::test(flavor = "current_thread")]
    async fn reserved_path_characters_cannot_replace_origin_or_base_path() {
        let fixture = Fixture::start();
        assert_eq!(
            fixture
                .http()
                .get(&["segment/with?query#fragment"], &[], &context(1000))
                .await
                .unwrap_err(),
            Error::Rejected
        );
        let events = fixture.events().await;
        assert_eq!(events["requests"], 1);
        assert_eq!(
            events["lastPath"],
            "/operator/segment%2Fwith%3Fquery%23fragment"
        );
        assert_eq!(events["target"], 0);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn duplicate_credential_parameter_and_dot_segments_are_rejected_before_io() {
        let fixture = Fixture::start();
        let http = Http::new(
            fixture
                .config(1024)
                .with_server_query_key("key", "synthetic-backend-secret".into())
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            http.get(&["ok"], &[("key", "replacement")], &context(1000))
                .await
                .unwrap_err(),
            Error::InvalidQuery
        );
        for segment in [".", ".."] {
            assert_eq!(
                http.get(&[segment], &[], &context(1000)).await.unwrap_err(),
                Error::InvalidQuery
            );
        }
        assert_eq!(fixture.events().await["requests"], 0);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn malformed_and_excessive_retry_delays_remain_sanitized_without_retry() {
        let fixture = Fixture::start();
        for path in ["rate-malformed", "rate-excess"] {
            assert_eq!(
                fixture
                    .http()
                    .get(&[path], &[], &context(1000))
                    .await
                    .unwrap_err(),
                Error::RateLimited {
                    retry_after_seconds: None
                }
            );
        }
        assert_eq!(fixture.events().await["requests"], 2);
    }
}
