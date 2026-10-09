// Controlled fault injection around a real SMTP service; never a production relay.
import tls from 'node:tls';
import fs from 'node:fs';
const sockets=new Set();const dropped=new Set();
const upstream=process.env.ROM_EXTRAS_SMTP_NATIVE_IP;
const fixture=process.env.ROM_EXTRAS_SMTP_NATIVE_FIXTURE;
const output=process.env.ROM_EXTRAS_SMTP_PROXY_RECORD;
const server=tls.createServer({key:fs.readFileSync(fixture+'/tls/server.key'),cert:fs.readFileSync(fixture+'/tls/server.pem'),minVersion:'TLSv1.2'},client=>{
 sockets.add(client);client.on('error',()=>{});client.on('close',()=>sockets.delete(client));
 const remote=tls.connect({host:upstream,port:1025,servername:'mailpit.fixture.test',ca:fs.readFileSync(fixture+'/tls/ca.pem'),minVersion:'TLSv1.2',rejectUnauthorized:true});
 sockets.add(remote);remote.on('error',()=>client.destroy());remote.on('close',()=>{sockets.delete(remote);client.destroy();});
 let tail='';let awaiting=false;let pending='';let subject='';let bodyMode=false;
 client.on('data',bytes=>{
  // Bound the inspection buffer; do not retain or record AUTH commands.
  tail+=bytes.toString('utf8');
  if(tail.length>1048576){client.destroy();remote.destroy();return;}
  while(tail.includes('\r\n')) {
   const end=tail.indexOf('\r\n');const line=tail.slice(0,end);tail=tail.slice(end+2);
   if(bodyMode&&line.startsWith('Subject: '))subject=line.slice(9);
   if(bodyMode&&line==='.'){awaiting=true;bodyMode=false;}
   if(line==='DATA')bodyMode=true;
  }
  remote.write(bytes);
 });
 remote.on('data',bytes=>{
  pending+=bytes.toString('utf8');
  if(pending.length>65536){client.destroy();remote.destroy();return;}
  while(pending.includes('\r\n')) {
   const end=pending.indexOf('\r\n');const line=pending.slice(0,end);pending=pending.slice(end+2);
   if(awaiting&&/^250 /.test(line)) {
    awaiting=false;
    if(!dropped.has(subject)) {
     dropped.add(subject);
     fs.appendFileSync(output,JSON.stringify({subject,native_data_acknowledgment_observed:true,acknowledgment_forwarded:false})+'\n');
     client.destroy();remote.destroy();return;
    }
   }
   client.write(line+'\r\n');
  }
 });
 client.on('close',()=>remote.destroy());
});
server.on('tlsClientError',()=>{});
server.listen(0,'127.0.0.1',()=>console.log(JSON.stringify({port:server.address().port})));
process.on('SIGTERM',()=>{for(const socket of sockets)socket.destroy();server.close(()=>process.exit(0));});
