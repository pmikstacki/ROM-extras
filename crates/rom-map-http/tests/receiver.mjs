import https from 'node:https';
import fs from 'node:fs';
const tls=process.env.ROM_EXTRAS_MAP_FIXTURE_TLS;
const state={requests:0,target:0,aborted:0,identified:false,backendKey:false,started:[],lastPath:null};
const server=https.createServer({cert:fs.readFileSync(`${tls}/node.pem`),key:fs.readFileSync(`${tls}/node.key`)},(req,res)=>{
  const u=new URL(req.url,'https://fixture.invalid');
  res.setHeader('Content-Type','application/json');
  if(u.pathname==='/operator/events'){res.end(JSON.stringify(state));return;}
  state.requests++;
  state.lastPath=u.pathname;
  state.started.push(performance.now());
  state.identified=req.headers['user-agent']==='ROM fixture/1 (operator@example.test)';
  state.backendKey=u.searchParams.get('key')==='synthetic-backend-secret';
  res.on('close',()=>{if(!res.writableEnded)state.aborted++;});
  switch(u.pathname){
    case '/operator/bad':res.writeHead(400);res.end('{"code":"NoRoute","message":"synthetic-secret"}');return;
    case '/operator/bad-huge':res.writeHead(400,{'Content-Length':'1048577'});res.end();return;
    case '/operator/bad-stream':res.writeHead(400);res.end('x'.repeat(1800));return;
    case '/operator/ok':res.end('[{"fixture":true}]');break;
    case '/operator/rate':res.writeHead(429,{'Retry-After':'2'});res.end('PRIVATE native error');break;
    case '/operator/rate-malformed':res.writeHead(429,{'Retry-After':'not-a-delay'});res.end('PRIVATE native error');break;
    case '/operator/rate-excess':res.writeHead(429,{'Retry-After':'3601'});res.end('PRIVATE native error');break;
    case '/operator/redirect':res.writeHead(302,{Location:`https://127.0.0.1:${server.address().port}/operator/target`});res.end('{}');break;
    case '/operator/target':state.target++;res.end('{}');break;
    case '/operator/declared':res.writeHead(200,{'Content-Length':'2048'});res.end('');break;
    case '/operator/stream':res.writeHead(200);res.write('x'.repeat(900));res.end('x'.repeat(900));break;
    case '/operator/slow':setTimeout(()=>{if(!res.destroyed)res.end('{}');},500);break;
    default:res.writeHead(404);res.end('{}');
  }
});
server.listen(0,'127.0.0.1',()=>process.stdout.write(`${server.address().port}\n`));
