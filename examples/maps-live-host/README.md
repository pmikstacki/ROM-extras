# Live map host example

This example connects public ROM reads to HTTP routes. It keeps provider selection, credentials, Resource scope and session resolution in the host.

`Host::new` requires a Runtime, exact Resource kind, selected keys and a `SessionResolver`. The resolver maps the application's authenticated request to a `HostSession`. Clients cannot supply an Actor or provider endpoint.

The session carries an opaque generation and a current-authority check. Reads and geocoding check authority before and after I/O. The browser must discard replies from earlier generations.

Enable selection with `with_selection`. The host callback receives the session and exact Resource key. It must serialize the generation check and selection with its own session store. Revocation after dispatch returns `unknown`, because the callback may have completed. Pre-dispatch refusal returns HTTP 403.

Enable geocoding with `with_geocoding`. Supply a configured provider implementing the optional geocoding capability. Required attribution and native result IDs remain in the suggestion response. Suggestions do not become Resource IDs or writes.

POST routes require the exact origin configured by the host. JSON request bodies are limited to 16 KiB. Resource reads and provider queries have a five-second total budget. Responses use `Cache-Control: no-store`. No request activates geolocation or browser tile connections. There is no automatic retry or provider fallback.

| Route | Input | Result |
| --- | --- | --- |
| `GET /points` | Host-resolved session | Current approved title and longitude/latitude, exact IDs and session generation |
| `POST /selection` | `id`, `generation` | `accepted`, `rejected` or `unknown` |
| `POST /geocode` | `text`, `limit`, `generation` | Suggestions with provenance, attribution and explicit accuracy |

The two POST capabilities remain disabled until the host configures them. HTTP errors contain fixed categories, without native bodies or provider credentials.

## Controlled local demo

The `demo` example binds only `127.0.0.1:55468`. It expects a controlled MapTiler-shaped HTTPS endpoint, fixture CA and SQLite path. It permits the browser origin `http://127.0.0.1:55467`.

```sh
cargo run --manifest-path examples/maps-live-host/Cargo.toml --example demo -- \
  https://127.0.0.1:PROVIDER_PORT/operator/ \
  /absolute/path/to/fixture-ca.pem \
  /absolute/path/to/demo.sqlite
```

The demo uses synthetic credentials and a synthetic session resolver. It is not production authentication. Its fixture controls create, revoke and recover sessions. The session cookie is HttpOnly and SameSite=Strict. The local HTTP cookie does not have the Secure flag. Do not deploy these controls or credentials.

The provider fixture must be started explicitly. No donated public server or demo service is a default backend. The frontend preparation remains private pending public packaging and portable runner integration.

## Verification scope

`./scripts/check-map-live-host` checks actual local HTTP with Runtime SQLite, session refusal and revocation, selection outcomes and controlled HTTPS geocoding. It checks body limits, stale generations and exact Resource identities. It also runs formatting and Clippy.

The check requires `ROM_EXTRAS_MAP_FIXTURE_TLS`. `./scripts/check-map-adapters` creates a controlled TLS fixture and invokes this check. These tests do not qualify the actual MapTiler service or production session authentication.

Affected checks, source review and the full local verifier passed. See [the scoped verification record](../../docs/verification/maps-live-host-2026-10-09.json).
