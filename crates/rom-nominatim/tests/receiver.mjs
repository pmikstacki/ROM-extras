import https from 'node:https';
import fs from 'node:fs';
const tls=process.env.ROM_EXTRAS_MAP_FIXTURE_TLS;
const server=https.createServer({cert:fs.readFileSync(`${tls}/node.pem`),key:fs.readFileSync(`${tls}/node.key`)},(req,res)=>{
 const u=new URL(req.url,'https://fixture.invalid');
 res.setHeader('Content-Type','application/json');
 if(u.pathname==='/operator/reverse') {
  if(u.searchParams.get('format')!=='jsonv2'||u.searchParams.get('addressdetails')!=='0'){res.writeHead(400);res.end('{}');return;}
  if(u.searchParams.get('lon')==='0'&&u.searchParams.get('lat')==='0'){res.end(JSON.stringify({error:'Unable to geocode'}));return;}
  if(u.searchParams.get('lon')!=='21'||u.searchParams.get('lat')!=='52'){res.writeHead(400);res.end('{}');return;}
  res.end(JSON.stringify({osm_type:'relation',osm_id:'00123',lon:'21',lat:'52',display_name:'Warsaw fixture',licence:'Data © OpenStreetMap contributors, ODbL 1.0.'}));return;
 }
 if(u.pathname==='/operator/search') {
  switch(u.searchParams.get('q')) {
   case 'empty':res.end('[]');return;
   case 'excess':res.end('[{},{}]');return;
   case 'invalid':res.end('{"error":"synthetic-secret"}');return;
   case 'huge':res.writeHead(200,{'Content-Length':'1048577'});res.end();return;
   case 'backend':res.writeHead(503);res.end('synthetic-secret');return;
   case 'rate':res.writeHead(429,{'Retry-After':'2'});res.end('synthetic-secret');return;
   case 'slow':setTimeout(()=>res.end('[]'),500);return;
  }
 }
 if(u.pathname!='/operator/search'||u.searchParams.get('format')!=='jsonv2'||u.searchParams.get('limit')!=='1'||u.searchParams.get('q')!=='Warsaw & test'||!req.headers['user-agent'].includes('operator@example.test')) {res.writeHead(400);res.end('{}');return;}
 res.end(JSON.stringify([{osm_type:'relation',osm_id:'00123',lon:'21',lat:'52',display_name:'Warsaw fixture',licence:'Data © OpenStreetMap contributors, ODbL 1.0.'}]));
});
server.listen(0,'127.0.0.1',()=>process.stdout.write(`${server.address().port}\n`));
