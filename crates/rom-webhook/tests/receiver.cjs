// Independent HTTPS receiver: uses Node crypto, never ROM-extras signing code.
const https = require('node:https');
const fs = require('node:fs');
const crypto = require('node:crypto');
const seen = new Set();
const counts = new Map();
const server = https.createServer({key:fs.readFileSync(process.argv[2]),cert:fs.readFileSync(process.argv[3])}, (req,res) => {
  if(req.method==='GET' && req.url==='/stats') {res.end([...counts].map(([path,count])=>`${path}=${count}`).concat(`deduplicated-effects=${seen.size}`).join('\n'));return;}
  counts.set(req.url,(counts.get(req.url)||0)+1);
  const chunks=[];
  req.on('data',chunk=>chunks.push(chunk));
  req.on('end',()=>{
    const body=Buffer.concat(chunks);
    const id=req.headers['webhook-id'], timestamp=req.headers['webhook-timestamp'];
    const expected='v1,'+crypto.createHmac('sha256',Buffer.alloc(32,7)).update(`${id}.${timestamp}.`).update(body).digest('base64');
    const got=Buffer.from(req.headers['webhook-signature']||'');
    if(req.method!=='POST'||id!=='work-17'||!/^180000000[01]$/.test(timestamp)||body.toString()!=='false'||got.length!==expected.length||!crypto.timingSafeEqual(got,Buffer.from(expected))) { res.writeHead(400); res.end(); return; }
    if(req.url==='/redirect') {res.writeHead(307,{Location:'/trap'});res.end();}
    else if(req.url==='/reject') {res.writeHead(400);res.end();}
    else if(req.url==='/server-error') {res.writeHead(500);res.end();}
    else if(req.url==='/disconnect') {req.socket.destroy();}
    else if(req.url==='/timeout') {setTimeout(()=>{res.writeHead(204);res.end();},5000);}
    else if(req.url==='/slow') {setTimeout(()=>{res.writeHead(204);res.end();},100);}
    else if(req.url==='/deduplicate') {seen.add(id);res.writeHead(seen.size===1?204:500);res.end();}
    else if(req.url==='/trap') {res.writeHead(204);res.end();}
    else {res.writeHead(204);res.end();}
  });
});
server.listen(0,'127.0.0.1',()=>process.stdout.write(`${server.address().port}\n`));
