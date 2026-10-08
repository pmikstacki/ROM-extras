const { test } = require('node:test');
const assert = require('node:assert/strict');
const { metadata, produce } = require('./protocol.cjs');
function i32(n) { const b = Buffer.alloc(4); b.writeInt32BE(n); return b; }
function broker() { return Buffer.concat([i32(1), Buffer.from([10]), Buffer.from('127.0.0.1'), i32(55449), Buffer.from([0, 0])]); }
test('flexible metadata rewrites only the broker port, with nullable rack and tags', () => {
  const packet = Buffer.concat([i32(17), Buffer.from([0]), i32(0), Buffer.from([2]), broker(), Buffer.from([0, 0, 0, 0])]);
  const before = Buffer.from(packet);
  metadata(packet, 12, 34567);
  const position = 4 + 1 + 4 + 1 + 4 + 1 + 9;
  assert.equal(packet.readInt32BE(position), 34567);
  before.writeInt32BE(34567, position);
  assert.deepEqual(packet, before);
});
test('metadata refuses another endpoint and truncated fields', () => {
  const packet = Buffer.concat([i32(17), Buffer.from([0]), i32(0), Buffer.from([2]), broker()]);
  packet.writeInt32BE(9999, packet.length - 6);
  assert.throws(() => metadata(packet, 12, 34567));
  assert.throws(() => metadata(Buffer.alloc(5), 12, 34567));
});
function report(error) {
  const part = Buffer.alloc(14); part.writeInt32BE(0); part.writeInt16BE(error, 4); part.writeBigInt64BE(7n, 6);
  return Buffer.concat([i32(17), Buffer.from([0, 2, 2]), Buffer.from('t'), Buffer.from([2]), part]);
}
test('drop witness requires successful Produce acknowledgement and real offset', () => {
  assert.deepEqual(produce(report(0), 12), { partition: 0, offset: '7' });
  assert.equal(produce(report(3), 12), null);
  assert.throws(() => produce(report(0).subarray(0, 12), 12));
});
