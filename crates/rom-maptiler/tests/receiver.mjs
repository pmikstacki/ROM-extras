// Authored protocol fixture: not a native MapTiler service.
import https from 'node:https';
import fs from 'node:fs';
import path from 'node:path';
import {mapRequest} from './maps_receiver.mjs';
const tls = process.env.ROM_EXTRAS_MAP_FIXTURE_TLS;
const feature = {type:'Feature',id:'municipality.00001',place_name:'Fixture',geometry:{type:'Point',coordinates:[21,52]}};
const collection = features => JSON.stringify({type:'FeatureCollection',attribution:'fixture credit',features});
const server = https.createServer({key:fs.readFileSync(path.join(tls,'node.key')),cert:fs.readFileSync(path.join(tls,'node.pem'))}, (req,res) => {
 const url = new URL(req.url,'https://fixture.invalid');
 if(mapRequest(req,res,url))return;
 const query = decodeURIComponent(url.pathname.slice('/operator/geocoding/'.length)).replace(/\.json$/,'');
 if(req.method!=='GET'||!url.pathname.startsWith('/operator/geocoding/')||url.searchParams.get('key')!=='synthetic-secret'||url.searchParams.get('limit')!=='1'||url.searchParams.has('proximity')) {res.writeHead(400);res.end();return;}
 res.setHeader('content-type','application/json');
 if(query==='redirect'){res.writeHead(302,{location:'/operator/geocoding/Warsaw%20%26%20test.json?key=synthetic-secret&limit=1&autocomplete=false'});res.end();return;}
 if(query==='bounded'){if(url.searchParams.get('bbox')!=='20,51,22,53'){res.writeHead(400);res.end();return;}res.end(collection([feature]));return;}
 if(query==='rate'){res.writeHead(429,{'retry-after':'2'});res.end('synthetic-secret');return;}
 if(query==='backend'){res.writeHead(503);res.end('synthetic-secret');return;}
 if(query==='huge'){res.writeHead(200,{'content-length':'1048577'});res.end();return;}
 if(query==='slow'){setTimeout(()=>res.end(collection([feature])),250);return;}
 if(query==='invalid'){res.end(collection([{...feature,geometry:{type:'Point',coordinates:[181,52]}}]));return;}
 if(query==='excess'){res.end(collection([feature,feature]));return;}
 if(query==='empty'||query==='0,0'){res.end(collection([]));return;}
 if(query==='Warsaw & test'&&url.searchParams.get('autocomplete')==='false'||query==='21,52'){res.end(collection([feature]));return;}
 res.writeHead(400);res.end();
});
server.listen(0,'127.0.0.1',()=>console.log(server.address().port));
