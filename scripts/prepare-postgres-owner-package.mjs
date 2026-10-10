import {readFileSync,writeFileSync} from 'node:fs';
import {join} from 'node:path';
import {preparePackagedConsumer} from './lib/package-consumer.mjs';
const consumer=preparePackagedConsumer({
 prefix:'postgres-owner',isolated:true,packages:['rom-sql-core','rom-pgwire','rom-postgres'],
 source:'tests/postgres-owner-consumer/src/main.rs',
 supportSources:['tests/postgres-owner-consumer/src/transport.rs','tests/postgres-owner-consumer/src/fixture.rs','tests/postgres-owner-consumer/src/cases.rs','tests/postgres-owner-consumer/src/admission.rs','tests/postgres-owner-consumer/src/restart.rs','tests/mssql-owner-consumer/src/wire_proxy.rs'],
 extraDependencies:'postgres = "=0.19.14"\ntokio = { version = "=1.53.1", features = ["rt", "net", "time"] }',
});
const source=readFileSync(join(consumer,'src/main.rs'),'utf8');
const pathLine='#[path = "../../mssql-owner-consumer/src/wire_proxy.rs"]\n';
if(source.split(pathLine).length!==2) throw new Error('Expected one shared relay source annotation');
writeFileSync(join(consumer,'src/main.rs'),source.replace(pathLine,''));
const lock=readFileSync('tests/postgres-owner-consumer/Cargo.lock','utf8');
writeFileSync(join(consumer,'Cargo.lock'),lock.replace('name = "postgres-owner-consumer"','name = "rom-postgres-owner-packaged-consumer"'));
process.stdout.write(consumer+'\n');
