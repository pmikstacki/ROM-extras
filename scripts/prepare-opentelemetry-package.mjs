import {preparePackagedConsumer} from './lib/package-consumer.mjs';
const pin='https://github.com/pmikstacki/ROM';
const rev='d7ef529040eec60dc869034c2d33130219db85fe';
const dependencies=['rom','rom-sqlite','rom-redb'].map(name=>`${name} = { version = "=0.0.3", git = "${pin}", rev = "${rev}"${name==='rom'?', default-features = false, features = ["derive"]':''} }`).join('\n');
process.stdout.write(preparePackagedConsumer({
 prefix:'opentelemetry',isolated:true,
 packages:['rom-opentelemetry'],
 source:'tests/opentelemetry-public-consumer/src/main.rs',
 supportSources:['tests/opentelemetry-public-consumer/src/capture.rs','tests/opentelemetry-public-consumer/src/host.rs'],
 extraDependencies:dependencies+'\ntokio = { version = "=1.53.1", features = ["rt"] }\nopentelemetry = { version = "=0.33.0", default-features = false, features = ["metrics"] }\nopentelemetry_sdk = { version = "=0.33.0", default-features = false, features = ["metrics"] }',
 extraPatches:`rom = { version = "=0.0.3", git = "${pin}", rev = "${rev}" }`,
})+'\n');
