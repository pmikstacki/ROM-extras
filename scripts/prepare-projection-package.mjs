import { preparePackagedConsumer } from "./lib/package-consumer.mjs";

const rom = `{ version = "=0.0.3", git = "https://github.com/pmikstacki/ROM", rev = "d7ef529040eec60dc869034c2d33130219db85fe" }`;
process.stdout.write(preparePackagedConsumer({
  prefix: "projection",
  packages: ["rom-sql-core", "rom-projection-core"],
  source: "tests/packaged-projections/main.rs",
  supportSources: ["tests/packaged-projections/checkpoint_case.rs"],
  extraDependencies: `rom = ${rom}
tokio = { version = "=1.53.1", default-features = false, features = ["rt"] }
serde_json = { version = "=1.0.151", features = ["arbitrary_precision", "preserve_order"] }`,
  extraPatches: `rom = ${rom}\nrom-backup = ${rom}`,
}) + "\n");
