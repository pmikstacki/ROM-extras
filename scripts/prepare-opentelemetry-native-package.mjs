import {preparePackagedConsumer} from './lib/package-consumer.mjs';
import {readFileSync,writeFileSync} from 'node:fs';
import {join} from 'node:path';
const pin='https://github.com/pmikstacki/ROM';
const rev='d7ef529040eec60dc869034c2d33130219db85fe';
const dependencies=['rom','rom-sqlite','rom-redb'].map(name=>`${name} = { version = "=0.0.3", git = "${pin}", rev = "${rev}"${name==='rom'?', default-features = false, features = ["derive"]':''} }`).join('\n');
const consumer=preparePackagedConsumer({
 prefix:'opentelemetry-native',isolated:true,packages:['rom-opentelemetry'],
 source:'tests/opentelemetry-native-consumer/src/main.rs',
 supportSources:['tests/opentelemetry-native-consumer/src/transport.rs','tests/opentelemetry-public-consumer/src/host.rs'],
 extraDependencies:dependencies+`\ntokio = { version = "=1.53.1", features = ["rt"] }
opentelemetry = { version = "=0.33.0", default-features = false, features = ["metrics", "trace"] }
opentelemetry_sdk = { version = "=0.33.0", default-features = false, features = ["metrics", "trace"] }
opentelemetry-otlp = { version = "=0.33.0", default-features = false, features = ["metrics", "trace", "http-proto", "reqwest-blocking-client", "reqwest-rustls"] }
reqwest = { version = "=0.13.5", default-features = false, features = ["blocking", "rustls"] }`,
 extraPatches:`rom = { version = "=0.0.3", git = "${pin}", rev = "${rev}" }`,
});
const manifest=join(consumer,'Cargo.toml');
writeFileSync(manifest,readFileSync(manifest,'utf8').replace('rom-opentelemetry = "=0.1.0-dev"','rom-opentelemetry = { version = "=0.1.0-dev", features = ["trace"] }'));
const source=join(consumer,'src/main.rs');
writeFileSync(source,readFileSync(source,'utf8').replace('#[path = "../../opentelemetry-public-consumer/src/host.rs"]\n',''));
// The native consumer has a separately audited transport graph; preserve those exact versions.
writeFileSync(join(consumer,'Cargo.lock'),readFileSync('tests/opentelemetry-native-consumer/Cargo.lock'));
process.stdout.write(consumer+'\n');
