import {readFileSync,writeFileSync} from 'node:fs';
import {join} from 'node:path';
import {preparePackagedConsumer} from './lib/package-consumer.mjs';
const consumer=preparePackagedConsumer({
 prefix:'oracle-owner',isolated:true,packages:['rom-sql-core','rom-oracle'],
 source:'tests/oracle-owner-consumer/src/main.rs',
 supportSources:['tests/oracle-owner-consumer/src/fixture.rs','tests/oracle-owner-consumer/src/cases.rs','tests/oracle-owner-consumer/src/restart.rs'],
 extraDependencies:'oracle = "=0.6.3"\ntokio = { version = "=1.53.1", features = ["rt"] }',
});
const lock=readFileSync('tests/oracle-owner-consumer/Cargo.lock','utf8');
writeFileSync(join(consumer,'Cargo.lock'),lock.replace('name = "oracle-owner-consumer"','name = "rom-oracle-owner-packaged-consumer"'));
process.stdout.write(consumer+'\n');
