import {spawn} from 'node:child_process';
import path from 'node:path';
const tls=process.env.ROM_EXTRAS_MAP_FIXTURE_TLS;
if(!tls)throw Error('Explicit controlled fixture TLS directory required');
const fixture=spawn('node',['crates/rom-nominatim/tests/receiver.mjs'],{stdio:['ignore','pipe','inherit']});
let consumer;
const timer=setTimeout(()=>{consumer?.kill();fixture.kill();process.exitCode=1;},10000);
fixture.on('error',()=>{clearTimeout(timer);process.exitCode=1;});
fixture.stdout.once('data',data=>{
 const port=Number(String(data).trim());
 if(!Number.isInteger(port)||port<1||port>65535){fixture.kill();clearTimeout(timer);process.exitCode=1;return;}
 consumer=spawn('target/map-adapter-consumer/debug/rom-map-adapter-consumer',[`https://127.0.0.1:${port}/operator/`,path.join(tls,'ca.pem')],{stdio:'inherit'});
 consumer.on('error',()=>{fixture.kill();clearTimeout(timer);process.exitCode=1;});
 consumer.on('exit',code=>{fixture.kill();clearTimeout(timer);process.exitCode=code??1;});
});
