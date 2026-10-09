import {spawn} from 'node:child_process';
import path from 'node:path';
const profiles = {
 nominatim: ['crates/rom-nominatim/tests/receiver.mjs', 'target/map-adapter-consumer/debug/rom-map-adapter-consumer'],
 maptiler: ['crates/rom-maptiler/tests/receiver.mjs', 'target/maptiler-public-consumer/debug/rom-maptiler-independent-consumer'],
 host: ['crates/rom-maptiler/tests/receiver.mjs', 'cargo'],
 livehost: ['crates/rom-maptiler/tests/receiver.mjs', 'cargo'],
};
const profile = profiles[process.argv[2] ?? 'nominatim'];
if (!profile || process.argv.length > 3) throw Error('Unknown controlled consumer profile');
const tls=process.env.ROM_EXTRAS_MAP_FIXTURE_TLS;
if(!tls)throw Error('Explicit controlled fixture TLS directory required');
const fixture=spawn('node',[profile[0]],{stdio:['ignore','pipe','inherit']});
let consumer;
const timer=setTimeout(()=>{consumer?.kill();fixture.kill();process.exitCode=1;},10000);
fixture.on('error',()=>{clearTimeout(timer);process.exitCode=1;});
fixture.stdout.once('data',data=>{
 const port=Number(String(data).trim());
 if(!Number.isInteger(port)||port<1||port>65535){fixture.kill();clearTimeout(timer);process.exitCode=1;return;}
 const endpoint = `https://127.0.0.1:${port}/operator/`;
 const ca = path.join(tls,'ca.pem');
 const livehost = process.argv[2] === 'livehost';
 const host = process.argv[2] === 'host' || livehost;
 const args = host ? ['test','--manifest-path',livehost ? 'examples/maps-live-host/Cargo.toml' : 'examples/maps-host/Cargo.toml','--locked','--offline','--target-dir',livehost ? 'target/maps-live-host' : 'target/maps-host','--test',livehost ? 'native_geocode' : 'https','--','--ignored'] : [endpoint,ca];
 consumer=spawn(profile[1],args,{stdio:'inherit',env:host?{...process.env,ROM_MAP_HOST_ENDPOINT:endpoint,ROM_MAP_HOST_CA:ca}:process.env});
 consumer.on('error',()=>{fixture.kill();clearTimeout(timer);process.exitCode=1;});
 consumer.on('exit',code=>{fixture.kill();clearTimeout(timer);process.exitCode=code??1;});
});
