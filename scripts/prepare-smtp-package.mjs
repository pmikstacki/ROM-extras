import {preparePackagedConsumer} from './lib/package-consumer.mjs';
const pin='https://github.com/pmikstacki/ROM';
const rev='d7ef529040eec60dc869034c2d33130219db85fe';
const dependencies=['rom','rom-sqlite','rom-redb'].map(name=>`${name} = { version = "=0.0.3", git = "${pin}", rev = "${rev}"${name==='rom'?', default-features = false, features = ["derive"]':''} }`).join('\n');
process.stdout.write(preparePackagedConsumer({
 prefix:'smtp',isolated:true,
 packages:['rom-delivery-core','rom-email-core','rom-smtp'],
 source:'tests/smtp-public-consumer/src/main.rs',
 supportSources:['tests/smtp-public-consumer/src/runtime.rs'],
 extraDependencies:dependencies+'\ntokio = { version = "=1.53.1", features = ["rt", "macros"] }',
 extraPatches:`rom = { version = "=0.0.3", git = "${pin}", rev = "${rev}" }`,
})+'\n');
