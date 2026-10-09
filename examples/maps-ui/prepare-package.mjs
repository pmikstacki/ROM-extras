import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import {fileURLToPath} from 'node:url';
const directory = path.dirname(fileURLToPath(import.meta.url));
const profile = JSON.parse(fs.readFileSync(path.join(directory,'qualified-rom-ui.json'),'utf8'));
if (process.argv.length !== 3) throw Error('Provide the qualified external rom-ui archive path');
const maximum = 16 * 1024 * 1024;
const descriptor = fs.openSync(process.argv[2], fs.constants.O_RDONLY | fs.constants.O_NONBLOCK);
let bytes;
try {
  const metadata = fs.fstatSync(descriptor);
  if (!metadata.isFile() || metadata.size > maximum) throw Error('The rom-ui archive exceeds the qualified file limit');
  const buffer = Buffer.alloc(metadata.size + 1);
  let total = 0;
  while (total < buffer.length) {
    const read = fs.readSync(descriptor, buffer, total, buffer.length - total, null);
    if (!read) break;
    total += read;
  }
  if (total !== metadata.size) throw Error('The rom-ui archive changed during preparation');
  bytes = buffer.subarray(0, total);
} finally { fs.closeSync(descriptor); }
if (crypto.createHash('sha256').update(bytes).digest('hex') !== profile.sha256) throw Error('The rom-ui archive does not match the qualified package');
const destination = path.join(directory,profile.archive);
if (path.resolve(process.argv[2]) !== destination) fs.writeFileSync(destination,bytes);
console.log('Qualified external rom-ui package prepared; run pnpm install --frozen-lockfile');
