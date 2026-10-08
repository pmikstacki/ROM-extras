// Independent HTTPS receiver, test-only Node 22.16 experimental SQLite API.
const https = require('node:https');
const fs = require('node:fs');
const crypto = require('node:crypto');
const {DatabaseSync} = require('node:sqlite');
const db = new DatabaseSync(process.argv[4]);
db.exec('PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; CREATE TABLE IF NOT EXISTS delivered(id TEXT PRIMARY KEY, body BLOB NOT NULL); CREATE TABLE IF NOT EXISTS attempts(path TEXT PRIMARY KEY, count INTEGER NOT NULL);');
const seen = new Set();
const counts = new Map();
const server = https.createServer({key:fs.readFileSync(process.argv[2]),cert:fs.readFileSync(process.argv[3])}, (req,res) => {
  if(req.method==='GET' && req.url==='/stats') {
    const durable=db.prepare('SELECT id, body FROM delivered ORDER BY id').all();
    const attempts=db.prepare('SELECT path,count FROM attempts ORDER BY path').all();
    res.end([...counts].map(([path,count])=>`${path}=${count}`).concat(`deduplicated-effects=${seen.size}`,`durable-effects=${durable.length}`,attempts.map(r=>`durable-path:${r.path}=${r.count}`),durable.map(r=>`durable-id:${r.id}=${Buffer.from(r.body).toString('base64')}`)).flat().join('\n'));return;
  }
  counts.set(req.url,(counts.get(req.url)||0)+1);
  const chunks=[];
  req.on('data',chunk=>chunks.push(chunk));
  req.on('end',()=>{
    const body=Buffer.concat(chunks);
    const id=req.headers['webhook-id'], timestamp=req.headers['webhook-timestamp'];
    const expected='v1,'+crypto.createHmac('sha256',Buffer.alloc(32,7)).update(`${id}.${timestamp}.`).update(body).digest('base64');
    const got=Buffer.from(req.headers['webhook-signature']||'');
    const runtime=req.url.startsWith('/runtime-');
    const validId=runtime ? /^[0-9a-f]{64}$/.test(id||'') : id==='work-17';
    if(req.method!=='POST'||!validId||!/^180000000[01]$/.test(timestamp)||body.toString()!=='false'||got.length!==expected.length||!crypto.timingSafeEqual(got,Buffer.from(expected))) { res.writeHead(400); res.end(); return; }
    if(runtime) {
      db.exec('BEGIN IMMEDIATE');
      const inserted=db.prepare('INSERT OR IGNORE INTO delivered(id,body) VALUES(?,?)').run(id,body).changes;
      db.prepare('INSERT INTO attempts(path,count) VALUES(?,1) ON CONFLICT(path) DO UPDATE SET count=count+1').run(req.url);
      const existing=Buffer.from(db.prepare('SELECT body FROM delivered WHERE id=?').get(id).body);
      db.exec('COMMIT');
      if(!existing.equals(body)) {res.writeHead(409);res.end();return;}
      if(req.url==='/runtime-always-disconnect'||(req.url==='/runtime-lost-ack'&&inserted===1)) {req.socket.destroy();return;}
      res.writeHead(204);res.end();return;
    }
    if(req.url==='/redirect') {res.writeHead(307,{Location:'/trap'});res.end();}
    else if(req.url==='/reject') {res.writeHead(400);res.end();}
    else if(req.url==='/server-error') {res.writeHead(500);res.end();}
    else if(req.url==='/disconnect') {req.socket.destroy();}
    else if(req.url==='/timeout') {setTimeout(()=>{res.writeHead(204);res.end();},5000);}
    else if(req.url==='/slow') {setTimeout(()=>{res.writeHead(204);res.end();},100);}
    else if(req.url==='/deduplicate') {seen.add(id);res.writeHead(seen.size===1?204:500);res.end();}
    else {res.writeHead(204);res.end();}
  });
});
server.listen(0,'127.0.0.1',()=>process.stdout.write(`${server.address().port}\n`));
