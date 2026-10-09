import https from 'node:https';
import {readFileSync,writeFileSync} from 'node:fs';
import {join} from 'node:path';
const [tls,credentials,report]=process.argv.slice(2);
const token=JSON.parse(readFileSync(credentials)).token;
const counts={};const sockets=new Set();
const save=()=>writeFileSync(report,JSON.stringify({counts},null,2)+'\n',{mode:0o600});
const server=https.createServer({key:readFileSync(join(tls,'node.key')),cert:readFileSync(join(tls,'node.pem'))},(req,res)=>{
 const name=req.url.slice(1);counts[name]=(counts[name]??0)+1;save();
 if(req.headers.authorization!==`Bearer ${token}`){res.writeHead(401);res.end('PRIVATE');return;}
 if(req.headers.accept!=='application/json'||req.headers['accept-encoding']!=='identity') {res.writeHead(400);res.end();return;}
 let body=Buffer.from('21');const headers={'Content-Type':'application/json','ETag':'"v1"'};
 const codes={'missing':404,'precondition':412,'rate':429,'rate-invalid':429,'unavailable':503,'unauthorized':401,'redirect':302,'no-content':204,'partial':206,'not-modified':304};
 if(name in codes) {if(name==='rate')headers['Retry-After']='3';if(name==='rate-invalid')headers['Retry-After']='3601';if(name==='redirect')headers.Location='/redirect-target';res.writeHead(codes[name],headers);res.end('PRIVATE');return;}
 if(name==='missing-etag')delete headers.ETag;
 if(name==='wrong-etag')headers.ETag='"v2"';
 if(name==='duplicate-etag')headers.ETag=['"v1"','"v1"'];
 if(name==='encoded')headers['Content-Encoding']='gzip';
 if(name==='mime')headers['Content-Type']='application/json; charset=utf-16';
 if(name==='charset')headers['Content-Type']='application/json; charset=utf-8';
 if(name==='changed')body=Buffer.from('22');
 if(name==='duplicate-json')body=Buffer.from('{"k":1,"k":2}');
 if(name==='invalid-utf8')body=Buffer.from([255]);
 if(name==='headers-count')for(let i=0;i<40;i++)headers[`X-Test-${i}`]='x';
 if(name==='headers-bytes')headers['X-Test-Large']='x'.repeat(9000);
 if(name==='hold-headers'||name==='cancel'||name==='concurrency')return;
 if(name==='hold-body') {res.writeHead(200,{...headers,'Content-Length':2});res.write('2');return;}
 if(name==='truncated') {res.writeHead(200,{...headers,'Content-Length':3});res.write('21');res.socket.end();return;}
 if(name==='declared-large') {res.writeHead(200,{...headers,'Content-Length':70000});res.end();return;}
 if(name==='chunked-large') {res.writeHead(200,headers);res.write(Buffer.alloc(35000,32));res.end(Buffer.alloc(35000,32));return;}
 if(name==='chunked'){res.writeHead(200,headers);res.write('2');res.end('1');return;}
 res.writeHead(200,{...headers,'Content-Length':body.length});res.end(body);
});
server.on('connection',socket=>{sockets.add(socket);socket.on('close',()=>sockets.delete(socket));});
server.on('tlsClientError',()=>{});
server.listen(0,'127.0.0.1',()=>process.stdout.write(String(server.address().port)+'\n'));
for(const signal of ['SIGINT','SIGTERM'])process.on(signal,()=>{save();for(const socket of sockets)socket.destroy();server.close(()=>process.exit(0));});
