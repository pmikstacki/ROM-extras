// Test-only Kafka response fields, from Apache Kafka4.3.1 message schemas.
class Cursor {
  constructor(data) { this.data = data; this.at = 0; }
  take(size) {
    if (!Number.isSafeInteger(size) || size < 0 || this.at + size > this.data.length) throw Error('truncated field');
    const at = this.at; this.at += size; return at;
  }
  i16() { return this.data.readInt16BE(this.take(2)); }
  i32() { return this.data.readInt32BE(this.take(4)); }
  variable() {
    let value = 0;
    for (let shift = 0; shift <= 28; shift += 7) {
      const byte = this.data[this.take(1)];
      value += (byte & 127) * 2 ** shift;
      if (!(byte & 128)) { if (value > 0xffffffff) throw Error('varint overflow'); return value; }
    }
    throw Error('oversized varint');
  }
  tags() {
    const count = this.variable(); if (count > 64) throw Error('tag bound');
    for (let i = 0; i < count; i++) { this.variable(); this.take(this.variable()); }
  }
  string(flexible, nullable = false) {
    const size = flexible ? this.variable() - 1 : this.i16();
    if (nullable && size === -1) return null;
    const start = this.take(size); return this.data.subarray(start, start + size).toString('utf8');
  }
  array(flexible) { const size = flexible ? this.variable() - 1 : this.i32(); if (size < 0 || size > 64) throw Error('array bound'); return size; }
}
function header(packet, version) {
  if (version < 0 || version > 13) throw Error('unsupported version');
  const c = new Cursor(packet); c.i32(); if (version >= 9) c.tags(); return c;
}
function metadata(packet, version, port) {
  const flexible = version >= 9;
  const c = header(packet, version);
  if (version >= 3) c.i32();
  if (c.array(flexible) !== 1) throw Error('single broker fixture required');
  if (c.i32() !== 1 || c.string(flexible) !== '127.0.0.1') throw Error('unexpected broker');
  const position = c.at;
  if (c.i32() !== 55449) throw Error('unexpected fixture port');
  if (version >= 1) c.string(flexible, true);
  if (flexible) c.tags();
  packet.writeInt32BE(port, position);
}
function produce(packet, version) {
  if (version < 3) throw Error('unsupported Produce version');
  const flexible = version >= 9;
  const c = header(packet, version);
  if (c.array(flexible) !== 1) throw Error('single topic fixture required');
  if (version >= 13) c.take(16); else c.string(flexible);
  if (c.array(flexible) !== 1) throw Error('single partition fixture required');
  const partition = c.i32(); const error = c.i16();
  const offset = packet.readBigInt64BE(c.take(8));
  return error === 0 && partition >= 0 && offset >= 0n ? { partition, offset: offset.toString() } : null;
}
module.exports = { metadata, produce };
