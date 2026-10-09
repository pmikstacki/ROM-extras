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

Thirty-six local contract tests passed. An independent public consumer compiled and executed outside workspace feature unification.
That consumer implements a custom raster provider through the public API.
These are core-contract checks, not actual geocoder/routing/tiles acceptance.
Run the affected core checks with:

```sh
./scripts/check-map-core
```

BrowserPolicy approves explicit HTTPS origins without backend credentials or automatic requests.
TileSource validates bounded TileJSON 3.0.0 metadata, original vector-layer identifiers and plain-text credits.
RasterSource and VectorSource reject metadata of the other source kind.
MapStyle admits a restricted MapLibre version-8 profile with approved raster/vector sources, sprites and glyphs.
Referenced TileJSON must be resolved explicitly before browser disclosure; native source overrides are validated and required credits are retained.
This profile rejects terrain, imports, font-faces and currently unimplemented root rendering properties.
It is not a complete renderer schema validator. The host must also enforce browser origin controls on redirects.

## Required provider behavior

| Provider | Required capabilities | Current acceptance |
| --- | --- | --- |
| Host configuration | Self-hosted TileJSON and MapLibre styles/tiles | Implementation pending |
| Nominatim | Search and reverse lookup | Implementation pending |
| OSRM | Route GeoJSON with explicit prepared profile | Implementation pending |
| MapTiler | Styles/tiles and search/reverse | Implementation pending |

Hosts must explicitly configure endpoints and authorize browser disclosure.
Backend credentials remain server-side. Browser-intended tokens require a separate explicit host contract.
A self-hosted root style can contain remote tile, glyph, sprite and image connections; source approval must cover them.
Treat native attribution HTML as untrusted. Preserve required credits through plain text and controlled links.

The public OSMF Nominatim policy differs from self-hosted service policy and excludes generic platform geocoding.
No public demo service is a default production backend.
MapTiler terms require written permission for map proxying and restrict server-side map-content caching.
Provider contracts determine rate, retries, cache retention and data attribution. No universal map cache is assumed.

## Remaining integration

Controlled protocol fixtures, concrete adapters, explicit browser-token grants and packaged map consumers remain to implement.
The active rom-ui gallery worktree declares rom-ui/maps in version 0.1.0-alpha.6.
The older rom-ui checkout remains at alpha.3 without that export.
The export and ResourceMap source were inspected; ROM-extras has not yet executed an installed frontend integration example.
Use the current package with pnpm and retain presentation in rom-ui.
The host will select providers, authorize ROM reads and disclose approved map data.
SQL and projection ports do not acquire map operations.

The full local verifier must pass on a frozen combined source before integration.
Current map changes remain a working increment; no map-service production support is claimed.

Current core and shared-transport evidence: [verification record](verification/maps-core-shared-http-2026-10-09.json).
This record separates passed affected checks from the running full verifier and unfinished adapters.
