import fs from 'node:fs';
import path from 'node:path';
import cp from 'node:child_process';
import {FixtureProcesses} from './map-fixture-processes.mjs';
import {receiver} from '../crates/rom-smtp/tests/receiver.mjs';
const root=path.resolve('.superpowers/smtp-protocol-fixtures');fs.mkdirSync(root,{recursive:true,mode:0o700});
const fixture=fs.mkdtempSync(root+'/run-');
function openssl(args){cp.execFileSync('openssl',args,{stdio:['ignore','pipe','pipe'],timeout:10000});}
openssl(['req','-x509','-newkey','rsa:2048','-noenc','-keyout',fixture+'/ca.key','-out',fixture+'/ca.pem','-days','7','-subj','/CN=SMTP controlled fixture CA']);
openssl(['req','-newkey','rsa:2048','-noenc','-keyout',fixture+'/server.key','-out',fixture+'/server.csr','-subj','/CN=smtp.fixture.test']);
fs.writeFileSync(fixture+'/server.ext','subjectAltName=DNS:smtp.fixture.test,IP:127.0.0.1\nextendedKeyUsage=serverAuth\n');
openssl(['x509','-req','-in',fixture+'/server.csr','-CA',fixture+'/ca.pem','-CAkey',fixture+'/ca.key','-CAcreateserial','-out',fixture+'/server.pem','-days','7','-extfile',fixture+'/server.ext']);
openssl(['x509','-in',fixture+'/ca.pem','-outform','DER','-out',fixture+'/ca.der']);
for(const file of ['ca.key','server.key'])fs.chmodSync(fixture+'/'+file,0o600);
const events=fixture+'/events.jsonl';fs.writeFileSync(events,'');
const r=receiver({key:fs.readFileSync(fixture+'/server.key'),cert:fs.readFileSync(fixture+'/server.pem'),events});
await new Promise(resolve=>r.server.listen(0,'127.0.0.1',resolve));
const scope=new FixtureProcesses();
const interrupted=()=>{scope.close().finally(()=>process.exit(1));};
process.once('SIGINT',interrupted);process.once('SIGTERM',interrupted);
try {
 const port=r.server.address().port;
 const child=scope.start('cargo',['test','--locked','-p','rom-smtp','--features','controlled-protocol','--test','protocol'],{stdio:'inherit',env:{...process.env,ROM_EXTRAS_SMTP_FIXTURE_PORT:String(port),ROM_EXTRAS_SMTP_FIXTURE_CA_DER:fixture+'/ca.der'}});
 await scope.wait(child);
 const rows=fs.readFileSync(events,'utf8').trim().split('\n').filter(Boolean).map(line=>JSON.parse(line));
 for(const subject of ['accept','transient','permanent','wrongpositive','loss','slow','cancel','held','release','ratefirst','rejectquit']) {
  if(rows.filter(x=>x.subject===subject).length!==1)throw Error('Unexpected submission count for '+subject);
 }
 for(const subject of ['badauth','overflow','precancel','competing','ratesecond','wrongdata','initreject'])if(rows.some(x=>x.subject===subject))throw Error('Unexpected rejected/local submission');
 if(!rows.every(x=>x.wire.includes('Content-Transfer-Encoding: base64\r\n') && Buffer.from(x.wire.split('\r\n\r\n')[1].replace(/\s/g,''),'base64').toString('utf8')==='protected-message\r\n.\r\n'))throw Error('SMTP framing changed decoded message content');
 console.log('Authored TLS SMTP protocol passed; exact observed attempt counts and decoded content verified; native service remains separate; retained '+fixture);
} finally {await scope.close();await r.close();process.off('SIGINT',interrupted);process.off('SIGTERM',interrupted);}
