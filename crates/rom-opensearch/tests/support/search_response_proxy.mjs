// Test-only verified relay: retain the actual bounded native search response until release.
import https from 'node:https';
import fs from 'node:fs';
const [root, control] = process.argv.slice(2);
const ca = fs.readFileSync(root + '/tls/ca.pem');
const mark = (name, value) => {
  fs.writeFileSync(control + '/' + name + '.tmp', value);
  fs.renameSync(control + '/' + name + '.tmp', control + '/' + name);
};
let requests = 0;
mark("requests", "0");
const server = https.createServer({ca, cert:fs.readFileSync(root+'/tls/node.pem'),
  key:fs.readFileSync(root+'/tls/node.key'), requestCert:true, rejectUnauthorized:true}, (request, response) => {
  mark("requests", String(++requests));
  const path = new URL(request.url, 'https://127.0.0.1').pathname;
  if (!/^\/rom_extras_[a-z0-9_]+\/_(mapping|settings|search)$/.test(path)) {
    response.writeHead(400); response.end(); return;
  }
  const search = request.method === 'POST' && path.endsWith('/_search');
  const upstream = https.request({hostname:'127.0.0.1', port:55460, path:request.url,
    method:request.method, headers:request.headers, ca,
    cert:fs.readFileSync(root+'/tls/projection-writer.pem'),
    key:fs.readFileSync(root+'/client-private/projection-writer.key'),
    rejectUnauthorized:true, timeout:5000}, reply => {
    if (!search) { response.writeHead(reply.statusCode, reply.headers); reply.pipe(response); return; }
    let bytes = 0;
    const chunks = [];
    reply.on('data', chunk => {
      bytes += chunk.length;
      if (bytes > 1048576) { upstream.destroy(); response.destroy(); return; }
      chunks.push(chunk);
    });
    reply.on('end', () => {
      const body = Buffer.concat(chunks);
      let value;
      try { value = JSON.parse(body); } catch { response.destroy(); return; }
      mark('held', JSON.stringify({http:reply.statusCode, timed_out:value.timed_out, candidates:value.hits?.hits?.length}));
      const expiry = setTimeout(() => response.destroy(), 5000);
      const release = setInterval(() => {
        if (!fs.existsSync(control+'/release')) return;
        clearInterval(release); clearTimeout(expiry);
        const headers = {...reply.headers, 'content-length':String(body.length)};
        delete headers['transfer-encoding'];
        response.writeHead(reply.statusCode, headers); response.end(body);
      }, 10);
      response.once('close', () => {clearInterval(release); clearTimeout(expiry);});
    });
  });
  upstream.on('error', () => response.destroy());
  upstream.on('timeout', () => upstream.destroy());
  let incoming = 0;
  request.on('data', chunk => {incoming += chunk.length; if (incoming > 1048576) {request.destroy(); upstream.destroy();}});
  request.pipe(upstream);
});
server.listen(0, '127.0.0.1', () => process.stdout.write(String(server.address().port)+'\n'));
