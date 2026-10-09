import https from 'node:https';import fs from 'node:fs';
const tls=process.env.ROM_EXTRAS_MAP_FIXTURE_TLS;
let requests=0;
const server=https.createServer({cert:fs.readFileSync(`${tls}/node.pem`),key:fs.readFileSync(`${tls}/node.key`)},(req,res)=>{
 const u=new URL(req.url,'https://fixture.invalid');res.setHeader('Content-Type','application/json');
 if(u.pathname==='/operator/events'){res.end(JSON.stringify({requests}));return;}
 requests++;
 const prefix='/operator/route/v1/car/';
 for(const [key,value]of [['radiuses','5;5'],['geometries','geojson'],['overview','full'],['alternatives','false'],['steps','false'],['generate_hints','false'],['skip_waypoints','true']]){
  if(u.searchParams.get(key)!==value){res.writeHead(400);res.end('{"code":"InvalidQuery","message":"synthetic-secret"}');return;}
 }
 if(!u.pathname.startsWith(prefix)){res.writeHead(400);res.end('{"code":"InvalidUrl"}');return;}
 const coordinates=decodeURIComponent(u.pathname.slice(prefix.length));
 const failure=coordinates.startsWith('0,0;')?'NoRoute':coordinates.startsWith('180,90;')?'NoSegment':coordinates.startsWith('-1,52;')?'InvalidQuery':null;
 if(failure){res.writeHead(400);res.end(JSON.stringify({code:failure,message:'synthetic-secret'}));return;}
 if(coordinates!=='21,52;21.1,52.1'){res.writeHead(400);res.end('{"code":"InvalidQuery"}');return;}
 res.end(JSON.stringify({code:'Ok',routes:[{distance:900,duration:60,geometry:{type:'LineString',coordinates:[[21,52],[21.1,52.1]]}}]}));
});server.listen(0,'127.0.0.1',()=>process.stdout.write(`${server.address().port}\n`));
