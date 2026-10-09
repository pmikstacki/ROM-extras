# ROM map host example preparation

This host example uses public ROM 0.0.3 and map-provider contracts. Run `./scripts/check-map-host` with the controlled TLS fixture variables.

The host supplies the actor, exact Resource kind, selected keys and current session authority callback. `read_session_points` checks that callback before and after the bounded read. Runtime checks row and field disclosure for each Resource. The host must serialize this decision with its own session changes before sending a response. This helper does not implement authentication or a session store.

Only title, longitude and latitude enter the approved point. Hidden, missing, invalid or deleted locations produce no point. Key mismatches and duplicate keys fail closed. The total deadline includes all reads. The input budget is 200 keys.

`query_suggestions` uses an explicitly selected capability. It checks host authority before and after the request. Suggestions preserve native provenance, accuracy and required credit. They do not become Resource identities or writes.

SQLite and redb Runtime tests cover disclosure, refusal, tombstones, invalid coordinates, input bounds, cancellation and session revocation. The controlled MapTiler-shaped HTTPS test covers native provenance and sanitized browser data. It does not qualify the actual MapTiler service.

The private frontend preparation displays a fixture snapshot generated from these authorized Runtime reads. It also displays a separate controlled-provider suggestion. It uses pnpm and the actual rom-ui ResourceMap component. Browser tests cover exact selection, unknown-outcome recovery, no automatic external connections and no geolocation.

The Rust host increment passed source review, affected checks and the full local verifier. See `docs/verification/maps-host-2026-10-09.json`.

Remaining work includes a runnable live host, current session changes in the browser, operator-approved provider configuration and frontend packaging. Snapshot tests do not prove live session behavior.
