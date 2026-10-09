# Map provider assessment

Source review date: 2026-10-09. This assessment records protocol research. It does not qualify a deployed service.

## Proposed capability boundaries

Keep map presentation, place search, reverse lookup, and routing as separate ROM-extras capabilities. Require explicit host configuration for each endpoint.
Do not install a public demonstration endpoint as a production default. Do not connect to providers when configuration is absent.
Keep credentials in server configuration. Expose a browser key only when the host explicitly authorizes browser use.
Do not infer location from the browser or IP address automatically. These are project requirements, not claims about provider defaults.

## Self-hosted map presentation

TileJSON 3.0.0 describes tile metadata. It requires `tilejson` and nonempty absolute `tiles` URLs.
Vector tiles also require `vector_layers`, with layer `id` and `fields`.
The `scheme` defaults to `xyz`; `tms` changes the tile Y direction.
Bounds use `[west, south, east, north]`. Center uses `[longitude, latitude, zoom]` in WGS84 degrees.
TileJSON caching must honor valid HTTP cache headers for manifests and tiles.
Attribution can contain HTML; the specification warns about XSS and tracking.
[TileJSON 3.0.0 specification](https://raw.githubusercontent.com/mapbox/tilejson-spec/master/3.0.0/README.md).

MapLibre styles use root style `version: 8`. Root properties include sources, sprite resources, and glyph URLs.
Vector and raster sources can contain TileJSON URLs or direct tile templates.
Therefore, a self-hosted style URL can still reference external resources.
[Style root](https://maplibre.org/maplibre-style-spec/root/), [source specification](https://maplibre.org/maplibre-style-spec/sources/).

Proposed policy: authorize every resource origin, including redirects, source manifests, tiles, glyphs, sprites, and image resources.
Treat attribution as untrusted content. Use sanitized text or controlled links.
Do not assume a renderer's defaults satisfy the project's connection policy.

MapLibre GL JS uses BSD-3-Clause terms, with bundled third-party notices.
The renderer license does not license a provider's tiles, fonts, sprites, or datasets.
[MapLibre license](https://raw.githubusercontent.com/maplibre/maplibre-gl-js/main/LICENSE.txt).

## Nominatim

The reviewed current manual identifies Nominatim 5.3.2.
Search accepts `/search?q=...&format=jsonv2`. Structured address parameters cannot be combined with `q`.
The maximum search limit is 40; fewer results can be returned.
Set the response format explicitly. Language and bounds affect results.
[Search API](https://nominatim.org/release-docs/latest/api/Search/).

Reverse lookup accepts `/reverse?lat=...&lon=...&format=jsonv2`, in WGS84 degrees.
It returns the nearest suitable indexed object, not an exact address calculation.
The selected object can belong to another street. Missing coverage can produce an error.
[Reverse API](https://nominatim.org/release-docs/latest/api/Reverse/).

JSON search results are arrays; reverse results are individual objects.
`lat` and `lon` can be strings. They describe the object's centroid.
`boundingbox` is `[south, north, west, east]`, unlike TileJSON bounds.
`importance` is a ranking value, not a calibrated probability of correctness.
`place_id` is an internal identifier that can change after reimport or between servers.
[Output formats](https://nominatim.org/release-docs/latest/api/Output/).

### Public service policy versus self-hosting

The OSMF policy applies to `nominatim.openstreetmap.org`, not instances operated by other organizations.
Public use has an aggregate application limit of one request per second.
Requests need an identifying User-Agent or Referer. Attribution is required.
Autocomplete and systematic extraction are prohibited. Clients must support service switching; caching is recommended.
The policy explicitly bars generic public geocoding offered by no-code, low-code, or vibe-coding platforms.
It permits deliberate developer decisions only within its stated conditions.
Do not submit confidential information to this public service.
[Public Nominatim usage policy](https://operations.osmfoundation.org/policies/nominatim/).

Proposed ROM-extras policy: require a host-operated or contracted endpoint. Leave the public endpoint unconfigured.
Set limits and cache retention from that operator's policy. Do not copy the public limit onto every self-hosted deployment.

The Nominatim repository provides GPL version 3 license text.
Service software licensing and geocoding dataset licensing are separate concerns.
[Nominatim COPYING](https://raw.githubusercontent.com/osm-search/Nominatim/master/COPYING).

## OSRM routing

The official latest release resolved to `v26.10.0` during review.
The website still exposes `v5.24.0` documentation; qualify against the deployed version's tagged protocol documentation.
[Release](https://github.com/Project-OSRM/osrm-backend/releases/tag/v26.10.0), [older website API](https://project-osrm.org/docs/v5.24.0/api/).

The route API accepts `GET /route/v1/{profile}/{longitude},{latitude};{longitude},{latitude}`.
Coordinates use longitude before latitude. Request `geometries=geojson` to avoid polyline decoding ambiguity.
Polyline encoding uses latitude before longitude, with precision 5 or 6 according to its format.
Route distance is meters. Duration is estimated seconds. Waypoints report snapped locations.
Profiles depend on the Lua profile used during data preparation; a URL label does not switch the prepared graph.
Check the JSON `code`, not only the HTTP status. Handle `NoRoute`, `NoSegment`, invalid inputs, and size limits.
Alternative routes are not guaranteed. An optional `data_version` reports source data time.
[OSRM v26.10.0 HTTP protocol](https://raw.githubusercontent.com/Project-OSRM/osrm-backend/v26.10.0/docs/http.md).

Proposed policy: require an operator endpoint and explicit supported profile mapping.
Use that operator's request limits. Invalidate route caches when graph or profile versions change.
Do not present estimated duration as live traffic evidence or guaranteed arrival time.
Do not substitute straight-line geometry after `NoRoute` unless the caller explicitly requests a distinct approximation.

OSRM backend has BSD-2-Clause terms. Its license does not remove source dataset attribution obligations.
[OSRM license](https://raw.githubusercontent.com/Project-OSRM/osrm-backend/master/LICENSE.TXT).

## MapTiler

Map styles use `/maps/{mapId}/style.json?key=...`.
Raster metadata uses `/maps/{mapId}/{tileSize/}/tiles.json?key=...`.
Raster tiles use `/maps/{mapId}/{tileSize/}/{z}/{x}/{y}{scale}.{format}`.
Vector tiles and their metadata have a separate Tiles API.
Keep map IDs and tile formats configurable; do not equate a style identifier with a vector tileset identifier.
[Maps API](https://docs.maptiler.com/cloud/api/maps/), [Tiles API](https://docs.maptiler.com/cloud/api/tiles/).

Geocoding uses `/geocoding/{query}.json`; reverse lookup uses `/geocoding/{longitude},{latitude}.json`.
Results are GeoJSON FeatureCollections. Coordinates and centers use `[longitude, latitude]`; bounds use `[west, south, east, north]`.
Limit ranges from 1 to 10. Reverse nearest-neighbor behavior changes with `types` and `limit`.
The `proximity=ip` option performs IP-based geolocation. Do not set it automatically.
`400` indicates invalid input; `403` can indicate a missing, invalid, or restricted key.
The response includes attribution and relevance. Do not reinterpret relevance as measured positional accuracy.
[Geocoding API](https://docs.maptiler.com/cloud/api/geocoding/).

### Browser keys and server credentials

MapTiler documents browser API keys restricted by Allowed HTTP origins.
New keys have no restrictions. Use separate keys for applications, platforms, staging, and production.
Production browser keys should have only the required host origins.
Origin restrictions limit browser use; they do not make the key secret.
Backend-only credentials belong in server configuration.
[API key guide](https://docs.maptiler.com/guides/credentials/api-key/).

Service tokens must remain private. They support backend request signing with HMAC-SHA256.
Never include a service token, its secret, or server credentials in a browser descriptor.
[Service token guide](https://docs.maptiler.com/guides/credentials/service-token/).

### Attribution and usage rights

General terms require MapTiler attribution linked to its copyright page.
The Free Account requires its logo. OSM-based maps also need OpenStreetMap attribution.
Attribution must remain readable. Search-result databases require attached or embedded attribution.
[General terms, section 6](https://www.maptiler.com/terms/).

Cloud terms permit temporary personal caching for one end-user.
Proxy use requires a written agreement. Server-side map-content caching is prohibited under the standard terms.
Map-content export and excessive tile bulk download require an agreement.
Search-service results can be used outside the service; geocoding bulk use has an attribution requirement.
Free-plan use is limited to noncommercial use and commercial research/development.
Limits depend on the subscription or custom contract.
[Cloud terms, sections 1, 5, 6, and 7](https://www.maptiler.com/terms/cloud/).

Proposed policy: disable shared MapTiler map caches. Require contractual authorization before enabling a host proxy.
Technical support for server tokens does not override proxy terms.
Prefer explicit browser keys for authorized direct map delivery. Keep geocoding credentials server-side when the host's agreement permits mediation.
Do not assign a universal MapTiler request-per-second value. Obtain the applicable plan limits from the host.

## Provider-specific transport policies

These are proposed implementation choices. The source review did not establish deployment capacity or universal retry requirements.

| Provider | Cache policy | Rate and retry policy |
| --- | --- | --- |
| Self-hosted tiles/styles | Honor HTTP validators and cache headers. Version manifests and assets together. | Operator limits. Bound retries to read requests and transient failures. |
| Self-hosted Nominatim | Operator retention policy; keys include query, language, bounds, and output format. | Operator aggregate budget. Avoid implicit autocomplete. |
| OSRM | Include endpoint, graph version, profile, coordinates, and route options. | Operator limits. Do not retry permanent protocol errors or `NoRoute`. |
| MapTiler map content | Personal device cache only under standard Cloud terms. No shared server cache. | Host plan budget. Do not retry `400` or `403` automatically. |
| MapTiler geocoding | Apply search-result rights and host retention policy separately from map-content rules. | Host plan budget; include language, types, bounds, and proximity in cache keys. |

Use bounded attempts and cancellation. Honor an applicable `Retry-After` response.
Do not fail over to another provider without host authorization. Retries must remain within the provider's aggregate budget.

## Dataset licensing and qualification evidence

OpenStreetMap data is licensed under ODbL. Public use requires attribution and notice of the license.
Adapted database distribution can invoke share-alike obligations. Self-hosting does not remove them.
[OpenStreetMap copyright and license](https://www.openstreetmap.org/copyright).

Use authored synthetic geometry and invented labels for protocol fixtures where possible.
Record each fixture's origin and license. Retain attribution for OSM-derived extracts.
Do not redistribute commercial tiles, fonts, screenshots, or response captures without applicable rights.
Provider service terms and each dataset license remain separate from adapter software licensing.

Protocol fixtures verify URL construction, coordinate conversion, parsing, errors, and transport policy.
They do not prove map coverage, correct addresses, routing quality, authentication, CORS, provider permission, or service availability.

Qualification needs an explicitly authorized endpoint and a recorded deployed version.
Record dataset source/date, graph profile, known-answer cases, empty results, request limits, attribution, and observed connections.
Separate locally simulated protocol evidence from actual-service qualification evidence.
No service requests, builds, deployment changes, or real-provider qualification were executed for this assessment.
