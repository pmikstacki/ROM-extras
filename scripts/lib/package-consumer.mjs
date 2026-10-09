import { mkdtempSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { resolve, join, basename } from "node:path";
import { fileURLToPath } from "node:url";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";

// Both callers consume Cargo's normalized archives, never their workspace manifests.
export function preparePackagedConsumer({ prefix, packages, source, extraDependencies = "", extraPatches = "", supportSources = [], isolated = false }) {
  const root = resolve(fileURLToPath(new URL("../..", import.meta.url)));
  const artifacts = join(root, ".superpowers");
  mkdirSync(artifacts, { recursive: true });
  const directory = mkdtempSync(join(isolated ? tmpdir() : artifacts, `${prefix}-packaged-`));
  const version = "0.1.0-dev";
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
name = "rom-${prefix}-packaged-consumer"
version = "0.0.0"
edition = "2024"
rust-version = "1.99"
publish = false
[workspace]
[dependencies]
${dependencies}
${extraDependencies}
[patch.crates-io]
${patches}
${extraPatches}
`);
  writeFileSync(join(consumer, "Cargo.lock"), readFileSync(join(root, "Cargo.lock")));
  writeFileSync(join(consumer, "src/main.rs"), readFileSync(join(root, source)));
  for (const support of supportSources) writeFileSync(join(consumer, "src", basename(support)), readFileSync(join(root, support)));
  writeFileSync(join(directory, "archives.json"), JSON.stringify({ version, hashes, consumer }, null, 2) + "\n");
  return consumer;
}
