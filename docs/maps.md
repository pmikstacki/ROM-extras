# Map providers

Map providers are part of the [expanded ROM-extras goal](goal.md).
The [implementation plan](superpowers/plans/2026-10-09-map-providers.md) covers separate capabilities, adapters, examples and acceptance.
The [provider research](research/map-providers-2026-10-09.md) records official API, license, attribution and usage constraints.

## Current local increment

rom-map-core implements validated WGS84 coordinates, bounds, route line geometry, explicit metres/seconds, and bounded geocoding values.
Longitude precedes latitude. Bounding boxes use west, south, east, north; a western longitude greater than the eastern longitude crosses the antimeridian.
Geocode feature bounds do not establish positional accuracy. Unknown accuracy remains explicit.
Results retain original provider identity and required plain-text attribution.
ResourceLocation retains the unchanged public ROM Key. It does not authorize selection or writes.

Styles, raster tiles, vector tiles, geocoding, reverse-geocoding and routing ports are independently selectable.
Missing capabilities return Unsupported. No provider, external connection or geolocation starts automatically.
RequestContext combines a finite total budget with monotonic asynchronous cancellation.
The total budget must wrap admission/rate waits and network I/O in each adapter.

The initial core verification record contains thirty-six local contract tests. An independent public consumer compiled and executed outside workspace feature unification.
That consumer implements a custom raster provider through the public API.
These are core-contract checks, not actual geocoder/routing/tiles acceptance.
Run the affected core checks with:

```sh
./scripts/check-map-core
```

BrowserPolicy approves explicit HTTPS origins without backend credentials or automatic requests.
TileSource validates bounded TileJSON 3.0.0 metadata, original vector-layer identifiers and plain-text credits.
The optional host/adapter `tileSize` extension preserves 128, 256, 512 or 1024 logical raster pixels.
`RasterTileSize` provides those explicit values. Invalid sizes and raster size hints on vector sources are rejected.
Absent sizes remain absent. Resolved manifests retain their size; a validated style size can override it.
This finite profile does not qualify raster contents or rendering. See [the source-based decision](research/map-raster-size-2026-10-09.md).
The raster-size increment passed affected checks and the full verifier. See [the scoped record and retained failure](verification/maps-raster-size-2026-10-09.json).
RasterSource and VectorSource reject metadata of the other source kind.
MapStyle admits a restricted MapLibre version-8 profile with approved raster/vector sources, sprites and glyphs.
Referenced TileJSON must be resolved explicitly before browser disclosure; native source overrides are validated and required credits are retained.
This profile rejects terrain, imports, font-faces and currently unimplemented root rendering properties.
It is not a complete renderer schema validator. The host must also enforce browser origin controls on redirects.

## Required provider behavior

| Provider | Required capabilities | Current acceptance |
| --- | --- | --- |
| Host configuration | Self-hosted TileJSON and MapLibre styles/tiles | Implemented; native static HTTPS metadata qualified; rendering and tile contents pending |
| Nominatim | Search and reverse lookup | Implemented; native qualification pending |
| OSRM | Route GeoJSON with explicit prepared profile | Implemented; authored driving graph and restart qualified; regional datasets and other modes pending |
| MapTiler | Search/reverse; style/raster/vector metadata | Controlled HTTPS, independent-consumer and full-verifier checks passed; actual service and rendering pending |

Hosts must explicitly configure endpoints and authorize browser disclosure.
Backend credentials remain server-side. Browser-intended tokens require a separate explicit host contract.
A self-hosted root style can contain remote tile, glyph, sprite and image connections; source approval must cover them.
Treat native attribution HTML as untrusted. Preserve required credits through plain text and controlled links.

The public OSMF Nominatim policy differs from self-hosted service policy and excludes generic platform geocoding.
No public demo service is a default production backend.
MapTiler terms require written permission for map proxying and restrict server-side map-content caching.
Provider contracts determine rate, retries, cache retention and data attribution. No universal map cache is assumed.

## Remaining integration

Nominatim search and reverse adapters now have controlled HTTPS protocol tests and an independent public consumer.
Native-service qualification and full family acceptance remain incomplete. Packaged map consumers check extracted crate compatibility.
The public pnpm example consumes the qualified rom-ui 0.1.0-alpha.7 archive and its rom-ui/maps export.
Its Chromium scenarios execute the public Rust host with Runtime SQLite and controlled provider HTTPS.
Presentation remains in rom-ui. The host selects providers, authorizes ROM reads and discloses approved map data.
SQL and projection ports do not acquire map operations.

The full local verifier must pass on a frozen combined source before integration.
Published increments retain scoped verifier records. No map-service production support is claimed.

Current core and shared-transport evidence: [verification record](verification/maps-core-shared-http-2026-10-09.json).
The record proves the previous core/projection increment passed its full verifier.
The map HTTP and Nominatim increment passed its combined local verifier on 324 unchanged runtime inputs.
See [executed scope and retained failures](verification/maps-nominatim-2026-10-09.json).

## Nominatim backend configuration

The host must select an HTTPS service, an identifying user agent, provenance and a minimum dispatch interval.
The adapter rejects nominatim.openstreetmap.org. It does not use donated infrastructure as a generic platform backend.
Configuration opens no connection. Queries use explicit jsonv2 output and validated coordinates.

```rust
use rom_nominatim::{Config, Nominatim};
use std::time::Duration;

let provider = Nominatim::new(Config::new(
    "https://geocoder.example.test/operator/",
    "Example host/1 (operator@example.test)",
    "example-private-nominatim",
    Duration::from_secs(1),
)?)?;
```

This configuration example requires an operator-provided service; it is not a deployed endpoint.
Use with_ca for a host-approved private CA. Never disable certificate verification.
The host must share adapter instances when enforcing an aggregate service rate limit.
Admission allows one concurrent operation. Each response is limited to 1 MiB and at most the requested result count.
RequestContext bounds the complete operation and supports cancellation. Redirects and automatic retries are disabled.
A 429 response returns a bounded Retry-After hint; the host decides whether to retry.
No response cache is provided. The operator must approve caching under the selected service's terms before adding one.

Results retain native licence text and the OpenStreetMap copyright link.
Accuracy remains Unknown; importance is not a measured positional uncertainty.
An OSM reference uses its native type and exact identifier.
When both OSM fields are absent, an exact place_id uses the nominatim-place/ namespace.
This fallback is service/import scoped and unstable across reimports. Keep provider provenance attached to the identifier.
A partial or malformed OSM reference is rejected.
No result identifier creates or replaces a ROM Resource key.

Search supports non-wrapping bounded viewboxes. Antimeridian viewboxes return Unsupported before any request.
Reverse supports one native result or the exact JSONv2 no-coverage object from Nominatim 5.3.2.
Other error objects are rejected without exposing their contents.
The adapter does not activate browser connections, geolocation, sessions or ROM mutations.

Run ./scripts/check-map-adapters for controlled TLS protocol and independent consumer checks.
Each gate run generates a private CA and separate server certificate in a new retained .superpowers directory.
Set ROM_EXTRAS_MAP_FIXTURE_TLS to use an existing controlled fixture directory.
The authored receiver is not a running Nominatim service; these tests do not qualify a real Nominatim deployment.

## OSRM routing adapter

`rom-osrm` implements only the Routing capability. The host supplies an HTTPS endpoint, user agent, graph provenance and attribution. Demo endpoints are rejected. The host must select a finite snapping radius with `Config::with_snap_radius(Metres)` before constructing `Osrm`. No unlimited radius is implicit.

The host binds the travel mode to the graph preparation profile. A URL profile name cannot change that profile. Requests use longitude first, full GeoJSON geometry, metres and seconds. Native `NoRoute` and `NoSegment` responses return an absent route. Other native errors remain closed errors; native messages are never displayed. HTTP response limits, cancellation, timeouts and rate admission use the map transport. No caching or automatic retry is enabled.

The independent consumer is `tests/osrm-public-consumer`. Supply an explicit controlled HTTPS endpoint and CA file as its two arguments. It requires the authored qualification graph and a 5 metre snapping radius. The prior private qualification used OSRM 26.10.0, five authored nodes, a driving graph and two disconnected roads. It passed before and after restart. These results do not qualify regional OSM datasets, cycling, walking, public deployments or UI integration. The promoted source passed the full local verifier and the native restart rerun. See [the scoped verification record](verification/maps-osrm-2026-10-09.json).

See the [official OSRM HTTP API](https://project-osrm.org/docs/v5.24.0/api/) for profile preparation, coordinate ordering, radius options and native response codes. The authored fixture is MIT data. Real OSM imports require their own attribution and license review.

## Configured source adapters

`rom-configured-maps` supplies separate raster, vector and style capabilities.
`ConfiguredRaster`, `ConfiguredVector` and `ConfiguredStyle` admit explicit host documents without network I/O.
`HostedRaster`, `HostedVector` and `HostedStyle` read one explicitly selected HTTPS document when the host invokes the capability.
The document name is a single ASCII filename of at most 256 bytes. Configure nested paths in the backend endpoint.

Pass private backend configuration through `rom_map_http::Config`. Pass browser origins through a separate `BrowserPolicy`.
Server query keys remain in the transport configuration. Query-bearing browser URLs are rejected by default.
`BrowserPolicy::with_public_query_token` explicitly approves one exact token and query field for an already approved service origin.
The host must verify that the token is intended for browsers and configure provider-side application-origin restrictions.
The adapter cannot verify those account settings. Backend keys never create this grant.

Styles require explicit attribution and pre-approved TileJSON bindings keyed by exact native style source identifier.
The adapter does not fetch nested manifests. The host must bind each manifest to its selected source and enforce browser redirect policy.
Required credits survive source inlining. The host remains responsible for dataset rights and approved browser disclosure.

Reads use bounded response sizes, verified TLS, cancellation and total deadlines. Redirects and automatic retries are disabled.
There is no adapter cache. Admission and rate limits apply per provider instance; the host must share instances for aggregate limits.

The independent consumer is `tests/configured-map-consumer`. Without arguments, it checks explicit configuration and optional capabilities.
With an explicit controlled endpoint and CA file, it checks the authored native metadata fixture.
`ROM_EXTRAS_MAP_METADATA_ENDPOINT` and `ROM_EXTRAS_MAP_METADATA_CA` opt into this fixture-specific qualification in map gates.
The fixture contains authored MIT metadata served by Nginx 1.29.8. Its consumer passed before and after restart.
This qualifies metadata delivery, source resolution, 429 and rejection of redirects. It does not qualify tile data, rendering or production deployments.

Run `./scripts/check-map-package` to check normalized archives outside the workspace.
The configured-source increment passed the full local verifier on frozen source. See [the scoped verification record](verification/maps-configured-2026-10-09.json).


## MapTiler geocoding preparation

`rom-maptiler` implements separate search and reverse capabilities through `rom-map-http`.
Configure an explicit HTTPS base endpoint, identifying user agent, provenance label, dispatch interval and reviewed attribution profile.
Supply the server key through `Config::with_server_key`. This does not grant a browser token.
Configure a private CA only when the host approves that trust root.
Construction makes no network request.

Forward search sends the native path, explicit limit and `autocomplete=false`.
A bounded query sends bbox values in west/south/east/north order.
Wrapped boxes are unsupported. Numeric coordinate pairs and semicolon batches are rejected by the text capability.
Reverse sends explicit longitude/latitude and limit one.
Neither operation requests IP proximity or geolocation.

Native IDs remain exact provenance. They do not become ROM Resource keys.
Accuracy remains unknown; relevance is not positional accuracy.
The host binds exact native attribution to reviewed plain text with `AttributionProfile`.
Unknown attribution is rejected. The host is responsible for equivalence and required credits.
The adapter adds no cache, automatic retry, browser grant or Resource write.

Controlled HTTPS tests cover empty results, coordinates, bbox, limits, 429, redirects, deadlines and cancellation.
The independent consumer is `tests/maptiler-public-consumer`.
These tests qualify authored protocol fixtures, not the actual MapTiler service or account permissions.
MapTiler map metadata is described below. Actual service and map rendering remain pending.
The combined increment passed the full local verifier. See [the verification record](verification/maps-maptiler-2026-10-09.json).

## MapTiler map metadata

`MapTilerStyle`, `MapTilerRaster` and `MapTilerVector` implement independent metadata capabilities.
They share `Arc<MapTilerMaps>` for aggregate single-operation admission and the host-selected dispatch interval.
The raster adapter selects the Maps API's explicit 256-pixel variant. The vector adapter selects a separate Tiles API tileset ID.
Literal service IDs are restricted to 1–128 ASCII letters, digits, hyphens and underscores. They are not normalized or treated as ROM keys.

`MapsConfig` uses an explicit HTTPS backend and a separate `MapMetadata` decoder.
`MapMetadata` combines a zeroized server-only key, reviewed `AttributionProfile` and explicit `MapBrowserGrant`.
The grant approves a browser-intended token for selected service origins. The host must configure application-origin restrictions in its account.
The server key cannot supply or be contained in the browser token.

The host must call `MapsConfig::with_backend_metadata_permission` only when its endpoint/account agreement permits backend metadata processing.
This declaration cannot establish provider-side permission. MapTiler's standard terms require an appropriate agreement for a customer proxy.
No server cache, tile proxy, bulk download or redistribution is implemented.

Only known tile, source, glyph and sprite URL fields undergo credential replacement.
Query-bearing native URLs require exactly one `key` pair matching the private server key.
Approved URLs retain their supported path placeholders and receive only the explicitly granted public token.
Other queries, credentials, fragments and unapproved origins are rejected. Literal and nested percent-encoded server-key disclosure is rejected.
Inspection is bounded to eight decoding passes; malformed UTF-8 and excessive nesting are rejected.

The host explicitly selects `TileJsonVersion`. Versions 2.0/2.1 retain a maximum zoom default and limit of 22; versions 2.2/3.0 use 30.
Normalization admits the strict supported metadata subset through the shared core. It does not weaken the core's TileJSON 3.0.0 validator.
Raster size is explicit. Original vector layer identifiers and approved plain-text credits remain attached to the descriptor.
Unknown native credit is rejected. Arbitrary attribution HTML is not stripped or rendered.

Styles retain the core's restricted MapLibre-v8 profile. The host must bind referenced manifests by exact native source ID.
`VectorSource::into_source` transfers already approved metadata into an owned style binding without re-decoding it.
No nested manifest, browser request, geolocation, session operation or Resource write starts automatically.

The independent consumer in `tests/maptiler-public-consumer/src/maps.rs` exercises actual authored HTTPS.
It checks three capabilities, native endpoint paths, defaults, credits, bounds, limits, 429, redirects, deadlines, cancellation and shared admission.
Repeated explicit reads demonstrate no adapter cache. The first 429 is returned without retry.
These fixtures do not establish actual account permissions, catalog compatibility, tile data, sprite/font content or rendering.
The implementation plan is [MapTiler metadata adapters](superpowers/plans/2026-10-09-maptiler-metadata.md).
The affected checks and full local verifier passed on unchanged source before integration.
See [the scoped verification record](verification/maps-maptiler-metadata-2026-10-09.json).

## Host read and suggestion example

The Rust host example is `examples/maps-host`. It uses only public ROM and map-provider interfaces.

The host supplies its Actor, selected kind and exact Resource keys. `read_session_points` checks current host authority before and after the read. Runtime checks current row and field disclosure. The example returns only title and validated longitude/latitude. Hidden or invalid locations are omitted. The input limit is 200 keys.

The host must serialize its response decision with session changes. The callback must read current host state. This helper does not implement authentication or a session store.

`query_suggestions` uses an explicitly selected capability. It checks host authority before and after provider I/O. Native IDs remain provenance. Suggestions do not create Resource IDs or writes. Keep required attribution when rendering the result.

The check covers actual SQLite/redb Runtime reads and an authored HTTPS MapTiler-shaped fixture. It also covers revocation, missing capability, cancellation and private-field omission. It does not qualify the actual MapTiler service.

The [pnpm frontend example](../examples/maps-ui/README.md) uses the published rom-ui alpha.7 archive.
Its public runner starts controlled HTTPS and the public Rust live-host demo. Synthetic sessions do not qualify production authentication.

The Rust host increment passed the full local verifier. See [the verification record](verification/maps-host-2026-10-09.json).

## Live HTTP host example

The [live host example](../examples/maps-live-host/README.md) exposes approved points, exact Resource selection and optional geocoding through HTTP.
The application supplies its session resolver, selected Resource keys, current authority check and provider configuration.
Reads and geocoding check authority before and after I/O. Selection callbacks receive the session and exact Resource key.
Revocation after selection dispatch returns an unknown outcome. The host must serialize selection with its session store.

POST routes require the configured browser origin and limit JSON bodies to 16 KiB.
Read and geocoding operations have a five-second budget. Responses use `Cache-Control: no-store` and fixed error categories.
Suggestions retain provenance and attribution. They do not become Resource IDs or writes.

`./scripts/check-map-live-host` checks actual local HTTP with Runtime SQLite and controlled HTTPS geocoding.
It covers session refusal, stale generations, body limits, current authorized reads and accepted/rejected/unknown selection outcomes.
The local demo uses synthetic authentication. These checks do not qualify production sessions or the actual MapTiler service.

The live-host increment passed the full local verifier on unchanged source. See [the scoped verification record](verification/maps-live-host-2026-10-09.json).

## Frontend consumer and controlled runner

`examples/maps-ui` consumes `rom-ui/maps` without introducing Svelte or MapLibre into Rust core.
The archive SHA-256, frozen pnpm lockfile and separate package-preparation check establish the selected artifact.
The browser receives only approved point fields and separate geocoding suggestions. It does not receive backend credentials.
The host resolves sessions and owns authorization, Resource scope, selection and writes.

`./scripts/check-map-ui` requires the explicit qualified UI archive and an installed Chromium executable.
The gate prepares controlled TLS, builds the public Rust host and runs type checks, unit tests and Chromium scenarios.
Three browser scenarios use actual local host HTTP and authored provider HTTPS. Two intercept responses to exercise empty results and failed controls.
The tests also check no automatic geolocation, no external browser request and no credential in browser request bodies.

The runner uses five-second readiness and ninety-second browser budgets. It stops only owned process groups and retains fixture databases.
Process tests verify actual descendant listener cleanup, startup failure, readiness bounds and deadlines.
The current runner scope is Linux. Production authentication, actual MapTiler service access and arbitrary style/tile rendering remain unqualified.

The frontend increment passed the full local verifier on unchanged source. See [the scoped verification record](verification/maps-ui-2026-10-09.json).
