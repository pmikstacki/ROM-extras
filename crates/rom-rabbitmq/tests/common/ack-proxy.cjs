// Test-only plaintext loopback proxy. Client credentials pass through without inspection.
const net = require('node:net');
const fs = require('node:fs');
const [evidence] = process.argv.slice(2);
let dropped = false;
const server = net.createServer(client => {
  const broker = net.connect(55445, '127.0.0.1');
  let buffered = Buffer.alloc(0);
  const close = () => { client.destroy(); broker.destroy(); };
  client.pipe(broker);
  client.on('error', close);
  broker.on('error', close);
  client.on('close', () => broker.destroy());
  broker.on('close', () => client.destroy());
  broker.on('data', chunk => {
    buffered = Buffer.concat([buffered, chunk]);
    if (buffered.length > 1048576) { close(); return; }
    while (buffered.length >= 7) {
      const size = buffered.readUInt32BE(3);
      if (size > 1048568) { close(); return; }
      const length = size + 8;
      if (buffered.length < length) return;
      const frame = buffered.subarray(0, length);
      buffered = buffered.subarray(length);
      if (frame[length - 1] !== 0xce) { close(); return; }
      // Method frame: basic.ack, channel 1, publisher delivery tag 1.
      if (!dropped && frame[0] === 1 && frame.readUInt16BE(1) === 1 && size === 13 &&
          frame.readUInt16BE(7) === 60 && frame.readUInt16BE(9) === 80 &&
          frame.readBigUInt64BE(11) === 1n && (frame[19] & 0xfe) === 0) {
        fs.writeFileSync(evidence, 'channel=1;tag=1\n', { flag: 'wx', mode: 0o600 });
        dropped = true;
      } else client.write(frame);
    }
  });
});
server.listen(0, '127.0.0.1', () => console.log(server.address().port));
