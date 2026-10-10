import {readFileSync,writeFileSync} from 'node:fs';
import {join} from 'node:path';
import {preparePackagedConsumer} from './lib/package-consumer.mjs';
const consumer=preparePackagedConsumer({
 prefix:'cockroach-owner',isolated:true,packages:['rom-sql-core','rom-pgwire','rom-cockroach'],
 source:'tests/cockroach-owner-consumer/src/main.rs',
 supportSources:['tests/cockroach-owner-consumer/src/fixture.rs','tests/cockroach-owner-consumer/src/cases.rs','tests/cockroach-owner-consumer/src/admission.rs','tests/cockroach-owner-consumer/src/restart.rs','tests/mssql-owner-consumer/src/wire_proxy.rs'],
 extraDependencies:'postgres = "=0.19.14"\ntokio = { version = "=1.53.1", features = ["rt", "net", "time"] }',
});
const source=readFileSync(join(consumer,'src/main.rs'),'utf8');
const pathLine='#[path = "../../mssql-owner-consumer/src/wire_proxy.rs"]\n';
if(source.split(pathLine).length!==2) throw new Error('Expected one shared relay source annotation');
writeFileSync(join(consumer,'src/main.rs'),source.replace(pathLine,''));
const lock=readFileSync('tests/cockroach-owner-consumer/Cargo.lock','utf8');
writeFileSync(join(consumer,'Cargo.lock'),lock.replace('name = "cockroach-owner-consumer"','name = "rom-cockroach-owner-packaged-consumer"'));
process.stdout.write(consumer+'\n');
