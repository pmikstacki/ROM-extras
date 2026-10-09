// Test-only verified-mTLS relay: lose actual acknowledgement or delay a real submitted Bulk request.
import https from 'node:https';
import fs from 'node:fs';
const root=process.argv[2], mode=process.argv[3]??'loss', control=process.argv[4];
const ca=fs.readFileSync(root+'/tls/ca.pem');
const mark=(name,value)=>{fs.writeFileSync(control+'/'+name+'.tmp',value);fs.renameSync(control+'/'+name+'.tmp',control+'/'+name);};
const server=https.createServer({ca,cert:fs.readFileSync(root+'/tls/node.pem'),key:fs.readFileSync(root+'/tls/node.key'),requestCert:true,rejectUnauthorized:true},(request,response)=>{
  const bulk=request.method==='POST' && request.url.includes('/_bulk');
  const forward=body=>{
    const headers={...request.headers};if(body){headers['content-length']=String(body.length);delete headers['transfer-encoding'];}
    const upstream=https.request({hostname:'127.0.0.1',port:55460,path:request.url,method:request.method,headers,ca,cert:fs.readFileSync(root+'/tls/projection-writer.pem'),key:fs.readFileSync(root+'/client-private/projection-writer.key'),rejectUnauthorized:true,timeout:5000},reply=>{
      if(bulk){const chunks=[];let bytes=0;reply.on('data',chunk=>{bytes+=chunk.length;if(bytes>1048576){upstream.destroy();return;}chunks.push(chunk);});reply.on('end',()=>{if(control){const value=JSON.parse(Buffer.concat(chunks));mark('done',JSON.stringify({http:reply.statusCode,status:value.items.map(item=>item.index.status)}));}response.destroy();});}
      else {response.writeHead(reply.statusCode,reply.headers);reply.pipe(response);}
    });
    upstream.on('error',()=>response.destroy());upstream.on('timeout',()=>upstream.destroy());if(body)upstream.end(body);else request.pipe(upstream);
  };
  if(bulk && mode==='delay') {const chunks=[];let bytes=0;request.on('data',chunk=>{bytes+=chunk.length;if(bytes>1048576){request.destroy();return;}chunks.push(chunk);});request.on('end',()=>{mark('held','accepted bounded body');setTimeout(()=>forward(Buffer.concat(chunks)),7000);});}
  else forward(null);
});
server.listen(0,'127.0.0.1',()=>process.stdout.write(String(server.address().port)+'\n'));
