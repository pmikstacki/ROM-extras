import { mkdtempSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { resolve, join } from "node:path";
import { fileURLToPath } from "node:url";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";

const root = resolve(fileURLToPath(new URL("..", import.meta.url)));
const artifacts = join(root, ".superpowers");
mkdirSync(artifacts, { recursive: true });
const directory = mkdtempSync(join(artifacts, "secrets-packaged-"));
const version = "0.1.0-dev";
const packages = ["rom-secrets", "rom-kms", "rom-openbao"];
const hashes = {};
for (const name of packages) {
  const archive = join(root, "target/package", `${name}-${version}.crate`);
  hashes[name] = createHash("sha256").update(readFileSync(archive)).digest("hex");
  execFileSync("tar", ["-xzf", archive, "-C", directory], { timeout: 20000 });
}
const consumer = join(directory, "consumer");
mkdirSync(join(consumer, "src"), { recursive: true });
const dependencies = packages.map((name) => `${name} = "=${version}"`).join("\n");
const patches = packages.map((name) => `${name} = { path = ${JSON.stringify(join(directory, `${name}-${version}`))} }`).join("\n");
writeFileSync(join(consumer, "Cargo.toml"), `[package]
name = "rom-secrets-packaged-consumer"
version = "0.0.0"
edition = "2024"
rust-version = "1.99"
publish = false
[workspace]
[dependencies]
${dependencies}
tokio = { version = "=1.53.1", features = ["rt", "macros"] }
serde_json = "=1.0.151"
[patch.crates-io]
${patches}
`);
writeFileSync(join(consumer, "Cargo.lock"), readFileSync(join(root, "Cargo.lock")));
writeFileSync(join(consumer, "src/main.rs"), readFileSync(join(root, "tests/packaged-secrets/main.rs")));
writeFileSync(join(directory, "archives.json"), JSON.stringify({ version, hashes, consumer }, null, 2) + "\n");
process.stdout.write(consumer + "\n");
