import { preparePackagedConsumer } from "./lib/package-consumer.mjs";

const rom = `{ version = "=0.0.3", git = "https://github.com/pmikstacki/ROM", rev = "d7ef529040eec60dc869034c2d33130219db85fe" }`;
process.stdout.write(preparePackagedConsumer({
  prefix: "projection",
  isolated: true,
  packages: ["rom-sql-core", "rom-projection-core", "rom-opensearch"],
  source: "tests/packaged-projections/main.rs",
  supportSources: ["crates/rom-opensearch/tests/support/generation_readiness.rs", "crates/rom-projection-core/tests/search_fixture/search_permissions.rs", "crates/rom-opensearch/tests/support/search_proxy.rs", "crates/rom-opensearch/tests/support/search_response_proxy.mjs", "crates/rom-opensearch/tests/support/fixture_process.rs", "crates/rom-opensearch/tests/support/search_case.rs", "crates/rom-projection-core/tests/search_fixture/native_case.rs", "tests/packaged-projections/checkpoint_case.rs", "tests/packaged-projections/worker_case.rs", "crates/rom-projection-core/tests/native_history_fixture/native_history_case.rs", "crates/rom-opensearch/tests/support/public_case.rs", "crates/rom-opensearch/tests/support/native_fixture.rs", "crates/rom-opensearch/tests/support/approved_document.rs"],
  extraDependencies: `reqwest = { version = "=0.13.5", default-features = false, features = ["rustls"] }
rom = ${rom}
rom-sqlite = ${rom}
rom-redb = ${rom}
tokio = { version = "=1.53.1", default-features = false, features = ["rt", "time", "macros"] }
serde_json = { version = "=1.0.151", features = ["arbitrary_precision", "preserve_order"] }`,
  extraPatches: `rom = ${rom}\nrom-backup = ${rom}\nrom-sqlite = ${rom}\nrom-redb = ${rom}`,
}) + "\n");
