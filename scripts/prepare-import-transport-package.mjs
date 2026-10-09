import {preparePackagedConsumer} from './lib/package-consumer.mjs';
import {readFileSync,writeFileSync} from 'node:fs';
import {join} from 'node:path';
const pin='https://github.com/pmikstacki/ROM';const rev='d7ef529040eec60dc869034c2d33130219db85fe';
const dependencies=['rom','rom-sqlite','rom-redb'].map(name=>`${name} = { version = "=0.0.3", git = "${pin}", rev = "${rev}"${name==='rom'?', default-features = false, features = ["derive"]':', features = ["test-support"]'} }`).join('\n');
const consumer=preparePackagedConsumer({
 prefix:'import-transport',isolated:true,packages:['rom-import','rom-import-transport'],
 source:'tests/import-transport-consumer/src/main.rs',
 supportSources:['tests/import-transport-consumer/src/protocol.rs','tests/import-transport-consumer/src/native.rs','tests/import-public-consumer/src/model.rs'],
 extraDependencies:dependencies+'\ntokio = { version = "=1.53.1", features = ["rt", "time", "macros", "sync"] }\nsha2 = "=0.10.9"\nserde_json = "=1.0.151"\nserde = { version = "=1.0.229", features = ["derive"] }',
 extraPatches:`rom = { version = "=0.0.3", git = "${pin}", rev = "${rev}" }`,
});
const manifest=join(consumer,'Cargo.toml');
writeFileSync(manifest,readFileSync(manifest,'utf8').replace('rom-import-transport = "=0.1.0-dev"','rom-import-transport = { version = "=0.1.0-dev", features = ["http"] }'));
const source=join(consumer,'src/main.rs');
writeFileSync(source,readFileSync(source,'utf8').replace('#[path = "../../import-public-consumer/src/model.rs"]\n',''));
writeFileSync(join(consumer,'Cargo.lock'),readFileSync('tests/import-transport-consumer/Cargo.lock'));
process.stdout.write(consumer+'\n');
