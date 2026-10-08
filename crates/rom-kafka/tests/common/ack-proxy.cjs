// Plaintext loopback fault fixture; no payloads or credentials are logged.
const net = require('node:net');
const fs = require('node:fs');
const { metadata, produce } = require('./protocol.cjs');
const [evidence] = process.argv.slice(2);
const limit = 2097152;
let dropped = false;
let connections = 0;
function frames(socket, receive, close) {
  let buffered = Buffer.alloc(0);
  socket.on('data', chunk => {
    try {
      if (buffered.length + chunk.length > limit) throw Error('buffer bound');
      buffered = Buffer.concat([buffered, chunk]);
      while (buffered.length >= 4) {
        const size = buffered.readInt32BE(0);
        if (size < 4 || size > limit - 4) throw Error('frame bound');
        if (buffered.length < size + 4) return;
        const frame = Buffer.from(buffered.subarray(0, size + 4));
        buffered = buffered.subarray(size + 4);
        receive(frame);
      }
    } catch { close(); }
  });
}
const server = net.createServer(client => {
  if (++connections > 8) { connections--; client.destroy(); return; }
  const broker = net.connect(55449, '127.0.0.1');
  const pending = new Map();
  const close = () => { client.destroy(); broker.destroy(); };
  const write = (socket, frame) => { if (socket.writableLength + frame.length > limit) close(); else socket.write(frame); };
  client.setTimeout(15000, close); broker.setTimeout(15000, close);
  client.on('error', close); broker.on('error', close);
  client.on('close', () => { connections--; broker.destroy(); });
  broker.on('close', () => client.destroy());
  frames(client, frame => {
    if (frame.length < 12 || pending.size >= 64) throw Error('request bound');
    const key = frame.readInt16BE(4), version = frame.readInt16BE(6), correlation = frame.readInt32BE(8);
    if (pending.has(correlation)) throw Error('duplicate correlation');
    pending.set(correlation, { key, version });
    write(broker, frame);
  }, close);
  frames(broker, frame => {
    const correlation = frame.readInt32BE(4);
    const request = pending.get(correlation);
    if (!request) throw Error('unknown correlation');
    pending.delete(correlation);
    const packet = frame.subarray(4);
    if (request.key === 3) metadata(packet, request.version, server.address().port);
    if (!dropped && request.key === 0) {
      const report = produce(packet, request.version);
      if (report) {
        fs.writeFileSync(evidence, JSON.stringify({ api: 0, version: request.version, correlation, ...report }), { flag: 'wx', mode: 0o600 });
        dropped = true; return;
      }
    }
    write(client, frame);
  }, close);
});
server.listen(0, '127.0.0.1', () => console.log(server.address().port));
