import { preparePackagedConsumer } from "./lib/package-consumer.mjs";

const rom = `{ version = "=0.0.3", git = "https://github.com/pmikstacki/ROM", rev = "d7ef529040eec60dc869034c2d33130219db85fe" }`;
process.stdout.write(preparePackagedConsumer({
  prefix: "projection",
  packages: ["rom-projection-core"],
  source: "tests/packaged-projections/main.rs",
  supportSources: ["tests/packaged-projections/checkpoint_case.rs"],
  extraDependencies: `rom = ${rom}`,
  extraPatches: `rom = ${rom}\nrom-backup = ${rom}`,
}) + "\n");
