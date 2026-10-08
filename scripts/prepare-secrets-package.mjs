import { preparePackagedConsumer } from "./lib/package-consumer.mjs";

process.stdout.write(preparePackagedConsumer({
  prefix: "secrets",
  packages: ["rom-secrets", "rom-kms", "rom-openbao"],
  source: "tests/packaged-secrets/main.rs",
  extraDependencies: `tokio = { version = "=1.53.1", features = ["rt", "macros"] }
serde_json = "=1.0.151"`,
}) + "\n");
