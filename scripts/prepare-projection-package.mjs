import { preparePackagedConsumer } from "./lib/package-consumer.mjs";

const rom = `{ version = "=0.0.3", git = "https://github.com/pmikstacki/ROM", rev = "d7ef529040eec60dc869034c2d33130219db85fe" }`;
process.stdout.write(preparePackagedConsumer({
  prefix: "projection",
  isolated: true,
  packages: ["rom-sql-core", "rom-projection-core", "rom-opensearch"],
  source: "tests/packaged-projections/main.rs",
  supportSources: ["tests/packaged-projections/checkpoint_case.rs", "tests/packaged-projections/worker_case.rs", "crates/rom-projection-core/tests/native_history_fixture/native_history_case.rs", "crates/rom-opensearch/tests/support/public_case.rs", "crates/rom-opensearch/tests/support/native_fixture.rs", "crates/rom-opensearch/tests/support/approved_document.rs"],
  extraDependencies: `rom = ${rom}
rom-sqlite = ${rom}
rom-redb = ${rom}
tokio = { version = "=1.53.1", default-features = false, features = ["rt", "time"] }
serde_json = { version = "=1.0.151", features = ["arbitrary_precision", "preserve_order"] }`,
  extraPatches: `rom = ${rom}\nrom-backup = ${rom}\nrom-sqlite = ${rom}\nrom-redb = ${rom}`,
}) + "\n");
