import {readFileSync,writeFileSync} from 'node:fs';
import {join} from 'node:path';
import {preparePackagedConsumer} from './lib/package-consumer.mjs';
const rom = `{ version = "=0.0.3", git = "https://github.com/pmikstacki/ROM", rev = "d7ef529040eec60dc869034c2d33130219db85fe" }`;
const consumer=preparePackagedConsumer({
 prefix:'qdrant-target',isolated:true,
 packages:['rom-sql-core','rom-projection-core','rom-projection-http','rom-qdrant'],
 source:'tests/qdrant-target-consumer/src/main.rs',
 supportSources:['tests/qdrant-target-consumer/src/native.rs','tests/qdrant-target-consumer/src/recovery.rs','tests/qdrant-target-consumer/src/vector.rs','crates/rom-projection-core/tests/search_fixture/search_permissions.rs'],
 extraPatches:`rom = ${rom}\nrom-backup = ${rom}\nrom-sqlite = ${rom}\nrom-redb = ${rom}`,
 extraDependencies:`rom = { version = "=0.0.3", git = "https://github.com/pmikstacki/ROM", rev = "d7ef529040eec60dc869034c2d33130219db85fe" }
rom-sqlite = { version = "=0.0.3", git = "https://github.com/pmikstacki/ROM", rev = "d7ef529040eec60dc869034c2d33130219db85fe" }
rom-redb = { version = "=0.0.3", git = "https://github.com/pmikstacki/ROM", rev = "d7ef529040eec60dc869034c2d33130219db85fe" }
reqwest = { version = "=0.13.5", default-features = false, features = ["rustls"] }
serde_json = "=1.0.151"
tokio = { version = "=1.53.1", features = ["rt", "macros", "time"] }`,
});
writeFileSync(join(consumer,'src/main.rs'),readFileSync(join(consumer,'src/main.rs'),'utf8').replace('../../../crates/rom-projection-core/tests/search_fixture/search_permissions.rs','search_permissions.rs'));
writeFileSync(join(consumer,'Cargo.lock'),readFileSync('tests/qdrant-target-consumer/Cargo.lock','utf8').replace('name = "qdrant-target-consumer"','name = "rom-qdrant-target-packaged-consumer"'));
process.stdout.write(consumer+'\n');
