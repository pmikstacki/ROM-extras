# ROM map host UI example

This frontend uses the public `rom-ui/maps` components. The Rust host owns sessions, authorization, Resource scope, selection and provider credentials. The browser displays only approved point fields and separate geocoding suggestions.

## Prepare the external UI package

Use the exact `rom-ui` archive identified by `qualified-rom-ui.json`. The preparation script checks its SHA-256 before copying it into this directory. The archive and installed dependencies are excluded from Git.

Get the archive from the [public alpha.7 release](https://github.com/pmikstacki/rom-ui/releases/tag/v0.1.0-alpha.7). The qualified release SHA-256 is `840e1d256b4186406486ec34fe66e75587ac9e10c58d4178f8c9c52e92118164`.

```sh
node prepare-package.mjs /absolute/path/to/rom-ui-0.1.0-alpha.7.tgz
corepack pnpm install --frozen-lockfile
corepack pnpm check
node --experimental-strip-types --test host-points.test.ts host-selection.test.ts
corepack pnpm exec vite build
```

The qualified archive is an external build artifact. A version label alone does not establish that an archive matches it. This example does not publish the artifact or qualify all rom-ui components.

## Run the controlled local host

Run these commands from the repository root in the first terminal:

```sh
set -e
source scripts/map-fixture-tls.sh
rom_map_fixture_tls
printf 'Fixture CA: %s/ca.pem\n' "$ROM_EXTRAS_MAP_FIXTURE_TLS"
node crates/rom-maptiler/tests/receiver.mjs
```

The controlled provider prints its HTTPS port. In another terminal, follow the [Rust demo instructions](../maps-live-host/README.md).
Use the printed port, fixture CA and a new SQLite path. The host binds `127.0.0.1:55468`.
Its permitted browser origin is `http://127.0.0.1:55467`.
In `examples/maps-ui`, start the prepared frontend:

```sh
ROM_LIVE_FIXTURE=1 corepack pnpm exec vite preview --host 127.0.0.1 --port 55467 --strictPort
```

The explicit fixture flag enables a loopback proxy for host routes. The flag does not choose an external map service. Click `Connect host fixture` to establish the synthetic local session. Click `Query host geocoder` to dispatch the provider query. There is no automatic geolocation or tile connection.

The map uses a blank style. Resource IDs remain exact. A suggestion's native ID is provenance, not a Resource ID. The host session cookie is HttpOnly. No provider key enters component props or frontend configuration.

Unknown selection outcomes block another selection until the host recovers authority. Revocation clears local points and selection immediately. The UI reports confirmed revocation only after the host acknowledges it. A failed revocation request leaves the host outcome unknown. A delayed session response cannot restore revoked authority. Transport loss after selection remains an unknown outcome.

Empty geocoding results clear earlier suggestions. Failed geocoding and control requests show fixed feedback without provider response details. Obsolete replies cannot restore old suggestions or overwrite a newer session's status.

## Verification scope

Three controlled Chromium scenarios previously passed against actual host HTTP and an authored HTTPS provider fixture. They cover authorized Runtime SQLite reads, exact Resource selection, attribution, cookie isolation, unknown recovery and delayed-session refusal.

Two additional browser scenarios intercept responses to exercise empty results and refused controls. They reproduced the reviewed failures before the fix. All five scenarios passed after the fix. The earlier navigation timeout under server load remains recorded. Intercepted responses do not qualify actual provider behavior.

```sh
ROM_EXTRAS_MAP_UI_ARCHIVE=/absolute/path/to/rom-ui-0.1.0-alpha.7.tgz ./scripts/check-map-ui
```

Run the gate from the repository root. It prepares controlled TLS, builds the public Rust demo and starts only local fixtures.
It uses pnpm frozen installation, type checks, unit tests, a production build and five controlled Chromium scenarios.
The runner has a five-second startup budget and a ninety-second browser budget. It closes owned process groups after failure or interruption.
Fixture databases remain in `.superpowers/maps-ui-browser-fixtures`. Other containers, listeners and historical data are not removed.

Install Playwright's Chromium explicitly with `corepack pnpm exec playwright install chromium` from this example directory.
Alternatively, set `ROM_CHROMIUM_PATH` to an approved local executable. Browser downloads are not automatic.
The runner is qualified on Linux. Windows process-group cleanup is unsupported.

Affected checks, independent source review and the full repository verifier passed. See [the scoped record](../../docs/verification/maps-ui-2026-10-09.json).
Synthetic session authentication and authored HTTPS fixtures do not qualify production authentication or the actual MapTiler service.
