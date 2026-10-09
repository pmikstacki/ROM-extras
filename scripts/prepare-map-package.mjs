import { preparePackagedConsumer } from "./lib/package-consumer.mjs";
const rom = '{ version = "=0.0.3", git = "https://github.com/pmikstacki/ROM", rev = "d7ef529040eec60dc869034c2d33130219db85fe" }';
process.stdout.write(preparePackagedConsumer({
 prefix: "map", isolated: true,
 packages: ["rom-map-core", "rom-map-http", "rom-nominatim", "rom-osrm", "rom-configured-maps"],
 source: "tests/configured-map-consumer/src/main.rs",
 extraDependencies: 'tokio = { version = "=1.53.1", default-features = false, features = ["rt", "macros"] }',
 extraPatches: 'rom = '+rom,
})+"\n");
