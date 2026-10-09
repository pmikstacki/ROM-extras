import {FixtureProcesses} from './map-fixture-processes.mjs';
import {mkdtempSync,writeFileSync,readFileSync} from 'node:fs';
import {join,resolve} from 'node:path';
import {randomBytes} from 'node:crypto';
const binary=resolve(process.argv[2]??'target/import-transport-consumer/debug/import-transport-consumer');
const tls=process.env.ROM_EXTRAS_MAP_FIXTURE_TLS;
if(!tls)throw Error('Explicit local TLS fixture required');
const root=mkdtempSync(resolve('.superpowers/import-protocol-'));
const credentials=join(root,'credentials.json'),report=join(root,'requests.json');
writeFileSync(credentials,JSON.stringify({token:randomBytes(24).toString('hex')}),{mode:0o600});
const processes=new FixtureProcesses();
let interrupted=false;
const stop=()=>{interrupted=true;void processes.close().catch(()=>{});};
process.on('SIGINT',stop);process.on('SIGTERM',stop);
try {
 const server=processes.start('node',['tests/import-transport-consumer/fixture.mjs',tls,credentials,report]);
 const port=Number(await processes.readyLine(server));
 if(!Number.isInteger(port)||port<1||port>65535)throw Error('Invalid fixture port');
 const cli=processes.start(binary,[`https://127.0.0.1:${port}`,join(tls,'ca.pem'),credentials],{stdio:'inherit',env:{...process.env,HTTPS_PROXY:'http://127.0.0.1:1',HTTP_PROXY:'http://127.0.0.1:1',ALL_PROXY:'http://127.0.0.1:1'}});
 await processes.wait(cli,20000);
 if(interrupted)throw Error('Interrupted fixture run');
 const {counts}=JSON.parse(readFileSync(report));
 for(const [name,count] of Object.entries(counts))if(count!==1)throw Error(`Automatic retry or duplicate fixture request: ${name}`);
 for(const name of ['redirect-target','pre-cancel','closed-cancel','untrusted'])if(counts[name])throw Error('Forbidden fixture request');
 for(const name of ['good','chunked','charset','cancel','hold-headers','hold-body','paced','concurrency'])if(counts[name]!==1)throw Error(`Required scenario absent: ${name}`);
 console.log(`Controlled TLS transcript verified: ${Object.keys(counts).length} single requests; no automatic retry/proxy/redirect/refetch`);
 console.log(`Evidence retained: ${root}`);
} finally {await processes.close();process.off('SIGINT',stop);process.off('SIGTERM',stop);}
