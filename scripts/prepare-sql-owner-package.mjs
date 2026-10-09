import {readFileSync,writeFileSync} from 'node:fs';
import {join} from 'node:path';
import {preparePackagedConsumer} from './lib/package-consumer.mjs';
const consumer=preparePackagedConsumer({
 prefix:'sql-owner',isolated:true,packages:['rom-sql-core'],
 source:'tests/sql-owner-consumer/src/main.rs',
 supportSources:['tests/sql-owner-consumer/src/driver.rs','tests/sql-owner-consumer/src/cases.rs'],
 extraDependencies:'postgres = "=0.19.14"',
});
const lock=readFileSync('tests/sql-owner-consumer/Cargo.lock','utf8');
writeFileSync(join(consumer,'Cargo.lock'),lock.replace('name = "sql-owner-consumer"','name = "rom-sql-owner-packaged-consumer"'));
process.stdout.write(consumer+'\n');
