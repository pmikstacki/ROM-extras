import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { preparePackagedConsumer } from "./lib/package-consumer.mjs";

const consumer = preparePackagedConsumer({
  prefix: "secrets",
  isolated: true,
  packages: ["rom-secrets", "rom-kms", "rom-openbao"],
  source: "tests/packaged-secrets/main.rs",
  supportSources: ["crates/rom-openbao/tests/fixture/migration.rs", "crates/rom-openbao/tests/fixture/administration.rs", "crates/rom-openbao/tests/fixture/lifecycle.rs"],
  extraDependencies: `tokio = { version = "=1.53.1", features = ["rt", "macros", "time"] }
reqwest = { version = "=0.13.5", default-features = false, features = ["rustls"] }
serde_json = "=1.0.151"`,
});
const main = join(consumer, "src/main.rs");
writeFileSync(main, readFileSync(main, "utf8").replaceAll("../../crates/rom-openbao/tests/fixture/", ""));
process.stdout.write(consumer + "\n");
