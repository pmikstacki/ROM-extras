# MapTiler metadata adapters

Implement the map capabilities in the approved map-provider scope. Preserve the geocoding API and SQL/projection boundaries.

Official sources: [Maps API](https://docs.maptiler.com/cloud/api/maps/), [Tiles API](https://docs.maptiler.com/cloud/api/tiles/), [keys](https://docs.maptiler.com/guides/credentials/api-key/), [Cloud terms](https://www.maptiler.com/terms/cloud/) and [credits](https://www.maptiler.com/copyright/).

1. Add shared private metadata admission with a separate explicit public browser grant. Keep server credentials zeroized and unexported.
2. Normalize only known resource URL fields. Require approved HTTPS origins and exact native server keys. Preserve supported path placeholders.
3. Admit explicitly selected TileJSON 2.0/2.1/2.2/3.0 subsets. Preserve their zoom defaults and validate bounds, geometry metadata, native layer IDs and credits.
4. Add independently selectable style, 256-pixel raster-map and vector-tileset adapters. Validate exact literal service IDs. Resolve style manifests through explicit host bindings.
5. Require host confirmation that its endpoint/account agreement permits backend metadata processing. This declaration cannot verify provider-side permissions.
6. Use shared bounded HTTP and whole-operation contexts. Provide no cache, redirect, automatic retry, geolocation or browser activation.
7. Add meaningful admission tests, actual controlled HTTPS cases and an independent public consumer. Keep account-backed and renderer qualification separate.
8. Run affected checks, request source review and run the full verifier on frozen source before integration.

TileJSON 2.0 and 2.1 default maximum zoom is 22 and restrict zoom to 22. Version 2.2 defaults to 30.
See the official [2.0](https://github.com/mapbox/tilejson-spec/blob/master/2.0.0/README.md), [2.1](https://github.com/mapbox/tilejson-spec/blob/master/2.1.0/README.md) and [2.2](https://github.com/mapbox/tilejson-spec/blob/master/2.2.0/README.md) contracts.
ROM uses a strict supported subset. Malformed optional values are rejected; arbitrary unknown fields are not disclosed.
The actual native version and catalog style returned by an account remain unqualified until an explicit service test establishes them.
