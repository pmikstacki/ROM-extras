// Test-only hostile verified-mTLS endpoints; counters contain no credentials or payloads.
import https from 'node:https';
import fs from 'node:fs';
const root=process.argv[2],mode=process.argv[3],control=process.argv[4];
const options={ca:fs.readFileSync(root+'/tls/ca.pem'),cert:fs.readFileSync(root+'/tls/node.pem'),key:fs.readFileSync(root+'/tls/node.key'),requestCert:true,rejectUnauthorized:true};
let source=0,target=0;
const record=()=>{fs.writeFileSync(control+'/counts.tmp',JSON.stringify({source,target}));fs.renameSync(control+'/counts.tmp',control+'/counts');};
record();
const destination=https.createServer(options,(request,response)=>{target++;record();response.end('{}');});
destination.listen(0,'127.0.0.1',()=>{
 const server=https.createServer(options,(request,response)=>{
  source++;record();request.resume();
  if(mode.startsWith('redirect')){response.writeHead(Number(mode.slice(8)),{location:'https://127.0.0.1:'+destination.address().port+'/private'});response.end();}
  else if(mode==='declared-large'){response.writeHead(200,{'content-length':'1048577'});response.flushHeaders();}
  else if(mode==='stream-large'){response.writeHead(200,{'content-type':'application/json'});for(let i=0;i<65;i++)response.write(' '.repeat(16384));response.end('{}');}
  else if(mode==='malformed'){response.end('{not valid JSON');}
  else if(mode==='slow-generation'){const path=new URL(request.url,'https://fixture').pathname;const name=path.split('/')[1];let value;if(request.method==='PUT')value={acknowledged:true};else if(path.endsWith('/_mapping'))value={[name]:{mappings:JSON.parse(fs.readFileSync(control+'/mapping.json'))}};else value={[name]:{settings:{index:{translog:{durability:'request'},uuid:'fixture-uuid'}}}};setTimeout(()=>response.end(JSON.stringify(value)),200);}
  else if(mode==='deadline'){/* retain the socket until the client deadline */}
  else {response.end('{}');}
 });
 server.listen(0,'127.0.0.1',()=>process.stdout.write(String(server.address().port)+'\n'));
});
