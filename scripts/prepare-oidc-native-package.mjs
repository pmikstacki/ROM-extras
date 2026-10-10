import {preparePackagedConsumer} from './lib/package-consumer.mjs';
import {readFileSync,writeFileSync} from 'node:fs';
const source='tests/oidc-native-consumer';
const manifest=readFileSync(source+'/Cargo.toml','utf8');
const extraDependencies=manifest.split('[dependencies]\n')[1].split('\n').filter(line=>!/^rom-oidc-presets =/.test(line)).join('\n');
const consumer=preparePackagedConsumer({prefix:'oidc-native',isolated:true,packages:['rom-oidc-presets'],source:source+'/src/main.rs',supportSources:[source+'/src/resources.rs'],extraDependencies,extraPatches:'rom-auth = { version = "=0.0.3", git = "https://github.com/pmikstacki/ROM", rev = "d7ef529040eec60dc869034c2d33130219db85fe" }'});
writeFileSync(consumer+'/Cargo.lock',readFileSync(source+'/Cargo.lock','utf8').replace('name = "rom-oidc-native-consumer"','name = "rom-oidc-native-packaged-consumer"'));
process.stdout.write(consumer+'\n');
