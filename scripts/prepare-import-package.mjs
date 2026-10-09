import {preparePackagedConsumer} from './lib/package-consumer.mjs';
const pin='https://github.com/pmikstacki/ROM';
const rev='d7ef529040eec60dc869034c2d33130219db85fe';
const dependencies=['rom','rom-sqlite','rom-redb'].map(name=>`${name} = { version = "=0.0.3", git = "${pin}", rev = "${rev}"${name==='rom'?', default-features = false, features = ["derive"]':''} }`).join('\n');
process.stdout.write(preparePackagedConsumer({
 prefix:'import',isolated:true,packages:['rom-import'],
 source:'tests/import-public-consumer/src/main.rs',
 supportSources:['tests/import-public-consumer/src/model.rs'],
 extraDependencies:dependencies+'\ntokio = { version = "=1.53.1", features = ["rt"] }\nsha2 = "=0.10.9"',
 extraPatches:`rom = { version = "=0.0.3", git = "${pin}", rev = "${rev}" }`,
})+'\n');
