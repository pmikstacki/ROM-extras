import https from 'node:https';import fs from 'node:fs';
const tls=process.env.ROM_EXTRAS_MAP_FIXTURE_TLS;
const server=https.createServer({key:fs.readFileSync(`${tls}/node.key`),cert:fs.readFileSync(`${tls}/node.pem`)},(req,res)=>{
 const url=new URL(req.url,'https://fixture.test');
 if(url.searchParams.get('key')!=='server-fixture-only'){res.writeHead(403);res.end();return;}
 if(url.pathname==='/limited.json'){res.writeHead(429,{'Retry-After':'2'});res.end();return;}
 if(url.pathname==='/large.json'){res.writeHead(200,{'Content-Length':'2048'});res.end('x'.repeat(2048));return;}
 if(url.pathname==='/vector.json'){res.writeHead(200,{'Content-Type':'application/json'});res.end(JSON.stringify({tilejson:'3.0.0',tiles:['https://tiles.operator.test/{z}/{x}/{y}.pbf'],vector_layers:[{id:'Roads/00001',fields:{}}]}));return;}
 if(url.pathname==='/style.json'||url.pathname==='/opaque-style.json'){res.writeHead(200,{'Content-Type':'application/json'});res.end(JSON.stringify(url.pathname==='/style.json'?{version:8,sources:{},layers:[{id:'Background/00001',type:'background'}]}:{version:8,sources:{Base:{type:'raster',url:'https://tiles.operator.test/source.json'}},layers:[{id:'Base',type:'raster',source:'Base'}]}));return;}
 const send=()=>{res.writeHead(200,{'Content-Type':'application/json'});res.end(JSON.stringify({tilejson:'3.0.0',tiles:[url.pathname==='/evil.json'?'https://evil.test/{z}/{x}/{y}.png':'https://tiles.operator.test/{z}/{x}/{y}.png']}));};
 if(url.pathname==='/slow.json')setTimeout(send,300);else send();
});server.listen(0,'127.0.0.1',()=>console.log(server.address().port));
