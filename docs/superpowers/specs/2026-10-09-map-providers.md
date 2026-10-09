# Generic map providers

The user authorized planning, implementation and verification on 2026-10-09.
The [primary research](../../research/map-providers-2026-10-09.md) records API and usage constraints.
This extends the existing ROM-extras goal and preserves its unfinished work.

## Boundaries

rom-map-core owns typed spatial values and independent source, geocoding, reverse-geocoding and routing ports.
Adapters own native API requests and response normalization. A provider can expose any subset of ports.
The host owns authorization, Resource selection, mutations, sessions and approved browser disclosure.
Keep exact public ROM Key values. No Svelte or MapLibre runtime dependency belongs in Rust core.
ROM remains pinned to public revision d7ef529040eec60dc869034c2d33130219db85fe.

## Typed data

Coordinates use WGS84 decimal degrees in longitude, latitude order.
Require finite longitude from -180 through 180 and latitude from -90 through 90.
Bounding boxes use west, south, east, north. West greater than east denotes antimeridian crossing; never silently reorder it.
Geocoding queries contain bounded nonempty text and an explicit result limit.
Results carry validated coordinates, original provider identity, attribution and provenance.
Accuracy is unknown unless the provider explicitly supplies an accuracy category or distance estimate.
Do not reinterpret Nominatim importance or feature extent as positional accuracy.
Routes use bounded valid GeoJSON line geometry. Distances are metres; durations are seconds.
Reject negative, nonfinite or incorrectly typed native quantities before disclosure.

## Providers and browser approval

Implement host-selected configured style/tile sources, Nominatim search/reverse, OSRM routing, and MapTiler source/geocoding.
Endpoints are required host configuration. No public demo endpoint is a default production service.
Backend keys are private server configuration, never a browser descriptor field.
Browser-intended tokens require explicit host approval and provider-required origin restrictions.
Validate or authorize transitive style, glyph, sprite, tile and TileJSON connections.
A root style on the host origin does not prove that its nested resources stay on that origin.
Descriptors do not initiate geolocation or network requests. UI activation remains explicit.

## Transport and policies

Use fixed host-approved endpoints, verified TLS, bounded streaming, no implicit redirects, sanitized errors and bounded total deadlines.
Cancellation must terminate pending requests and rate waits. It is not merely a flag checked before I/O.
Reject 429 distinctly, retain a bounded Retry-After hint, and never retry implicitly.
Rate and cache policy must follow the selected service contract. Do not apply a universal shared map cache.
Public OSMF Nominatim policy differs from self-hosted Nominatim.
MapTiler proxy and server map-content cache require separate contractual permission.
Do not weaken projection transport to add map-specific headers, query authentication, policies or error categories.

Initial admission limits: 1024-byte search text, 1 through 40 geocode results with provider-specific stricter limits,
2 through 25 route waypoints, 8192 route vertices, and 1 MiB wire responses.
All configured operation deadlines are finite, from 1 millisecond through 60 seconds, default 5 seconds.
These are application bounds, not vendor maximums; do not truncate accepted input or output.

## Acceptance

Run boundary and malicious-response tests, real controlled protocol requests, independent public and extracted-archive consumers.
Cover empty results, malformed geometry, bounds, 429, timeout, cancellation, absent capability and secret leakage.
Controlled native-shaped HTTP fixtures do not qualify actual provider deployment or result quality.
Run affected checks and the full local verifier before integration.
Provide configuration/capability/attribution documentation and a pnpm ROM host/rom-ui/maps example.
The current rom-ui checkout does not export maps yet; verify its actual future exports instead of inventing a working import.
