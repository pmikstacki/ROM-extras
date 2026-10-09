# Raster tile size admission

Sources read on 2026-10-09:

- [MapLibre raster sources](https://maplibre.org/maplibre-style-spec/sources/#raster) define `tileSize` in pixels, with a default of 512.
- [MapTiler Maps API](https://docs.maptiler.com/cloud/api/maps/) defines a distinct 256 path for raster tiles and TileJSON.

A standalone raster descriptor must preserve an explicit size before a host passes it to MapLibre.
Otherwise the renderer can use its default for a 256-pixel source.
Device pixel ratio and the `@2x` transport variant do not establish a different logical tile size.

Keep ROM's existing finite admission profile: 128, 256, 512 and 1024 logical pixels.
These values are the supported ROM profile, not an exhaustive limit from MapLibre's number schema.
Expose `RasterTileSize` with explicit pixel units. Use one validator for standalone, inline and resolved sources.
Reject malformed sizes and vector sources with a raster size. Preserve absence without inventing metadata.

The optional `tileSize` field is a host/adapter admission extension to TileJSON 3.0.0.
This change does not establish the native TileJSON version returned by a MapTiler account.
MapTiler adapters must bind their selected endpoint profile to an explicit size.
The adapter must not infer size from an arbitrary tile filename or raster image dimensions.

Contract tests qualify approved JSON descriptors. They do not qualify raster contents, rendering or account-backed MapTiler responses.
