import {readFileSync,writeFileSync} from 'node:fs';
import {join} from 'node:path';
import {preparePackagedConsumer} from './lib/package-consumer.mjs';
const consumer=preparePackagedConsumer({
 prefix:'mssql-owner',isolated:true,packages:['rom-sql-core','rom-mssql'],
 source:'tests/mssql-owner-consumer/src/main.rs',
 supportSources:['tests/mssql-owner-consumer/src/fixture.rs','tests/mssql-owner-consumer/src/cases.rs','tests/mssql-owner-consumer/src/deadlock.rs','tests/mssql-owner-consumer/src/restart.rs'],
 extraDependencies:`tiberius = { version = "=0.13.0", default-features = false, features = ["tds73", "rustls"] }
tokio = { version = "=1.53.1", features = ["rt", "net", "time"] }
tokio-util = { version = "=0.7.19", features = ["compat"] }`,
});
const lock=readFileSync('tests/mssql-owner-consumer/Cargo.lock','utf8');
writeFileSync(join(consumer,'Cargo.lock'),lock.replace('name = "mssql-owner-consumer"','name = "rom-mssql-owner-packaged-consumer"'));
process.stdout.write(consumer+'\n');
