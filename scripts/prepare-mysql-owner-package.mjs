import {readFileSync,writeFileSync} from 'node:fs';
import {join} from 'node:path';
import {preparePackagedConsumer} from './lib/package-consumer.mjs';
const consumer=preparePackagedConsumer({
 prefix:'mysql-owner',isolated:true,packages:['rom-sql-core','rom-mysql'],
 source:'tests/mysql-owner-consumer/src/main.rs',
 supportSources:['tests/mysql-owner-consumer/src/fixture.rs','tests/mysql-owner-consumer/src/cases.rs','tests/mysql-owner-consumer/src/admission.rs','tests/mssql-owner-consumer/src/wire_proxy.rs'],
 extraDependencies:'mysql = {version="=28.0.3",default-features=false,features=["minimal-rust","native-tls"]}\ntokio = {version="=1.53.1",features=["rt","net","time"]}',
});
const source=readFileSync(join(consumer,'src/main.rs'),'utf8');
const pathLine='#[path = "../../mssql-owner-consumer/src/wire_proxy.rs"]\n';
if(source.split(pathLine).length!==2) throw new Error('Expected one shared relay source annotation');
writeFileSync(join(consumer,'src/main.rs'),source.replace(pathLine,''));
writeFileSync(join(consumer,'Cargo.lock'),readFileSync('tests/mysql-owner-consumer/Cargo.lock','utf8').replace('name = "mysql-owner-consumer"','name = "rom-mysql-owner-packaged-consumer"'));
process.stdout.write(consumer+'\n');
