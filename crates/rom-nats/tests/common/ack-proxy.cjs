// Test-only plaintext loopback proxy. Never inspect or log client CONNECT credentials.
const net = require('node:net');
const fs = require('node:fs');
const [stream, evidence, portText] = process.argv.slice(2);
if (!/^[A-Za-z0-9_-]+$/.test(stream)) throw new Error('invalid fixture stream');
const port=Number(portText);
if(!Number.isInteger(port)||port<1||port>65535)throw new Error('invalid fixture port');
let dropped = false;
const server = net.createServer(client => {
  const broker = net.connect(port, '127.0.0.1');
  let buffered = Buffer.alloc(0);
  client.pipe(broker);
  client.on('error', () => broker.destroy());
  broker.on('error', () => client.destroy());
  client.on('close', () => broker.destroy());
  broker.on('close', () => client.destroy());
  broker.on('data', chunk => {
    buffered = Buffer.concat([buffered, chunk]);
    if (buffered.length > 1048576) { client.destroy(); broker.destroy(); return; }
    while (true) {
      const end = buffered.indexOf('\r\n');
      if (end < 0) return;
      const tokens = buffered.subarray(0, end).toString('ascii').split(/\s+/);
      const message = tokens[0] === 'MSG' || tokens[0] === 'HMSG';
      const size = message ? Number(tokens.at(-1)) : 0;
      const headers = tokens[0] === 'HMSG' ? Number(tokens.at(-2)) : 0;
      if (!Number.isSafeInteger(size) || size < 0 || size > 1048576 || !Number.isSafeInteger(headers) || headers < 0 || headers > size) {
        client.destroy(); broker.destroy(); return;
      }
      const length = end + 2 + (message ? size + 2 : 0);
      if (buffered.length < length) return;
      const frame = buffered.subarray(0, length);
      buffered = buffered.subarray(length);
      let suppress = false;
      if (message && !dropped) {
        try {
          const ack = JSON.parse(frame.subarray(end + 2 + headers, end + 2 + size).toString('utf8'));
          if (ack.stream === stream && Number.isSafeInteger(ack.seq) && ack.seq > 0 && !ack.error) {
            fs.writeFileSync(evidence, `${ack.stream}:${ack.seq}\n`, { flag: 'wx', mode: 0o600 });
            dropped = true;
            suppress = true;
          }
        } catch (error) {
          // API response payloads and non-JSON messages pass unchanged. Evidence I/O errors fail closed.
          if (error.code) { client.destroy(); broker.destroy(); return; }
        }
      }
      if (!suppress) client.write(frame);
    }
  });
});
server.listen(0, '127.0.0.1', () => console.log(server.address().port));
