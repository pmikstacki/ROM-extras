import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));
const fixtures = path.join(root, '.superpowers/maps-ui-package-fixtures');
fs.mkdirSync(fixtures, {recursive:true});
function prepare(archive) {
  return spawnSync(process.execPath, ['examples/maps-ui/prepare-package.mjs', archive], {cwd:root, encoding:'utf8', timeout:5000});
}
test('matching version label cannot admit a corrupt external archive', () => {
  const directory = fs.mkdtempSync(path.join(fixtures, 'corrupt-'));
  const archive = path.join(directory, 'rom-ui-0.1.0-alpha.7.tgz');
  fs.writeFileSync(archive, 'Not the qualified package');
  const result = prepare(archive);
  assert.equal(result.status, 1);
  assert.match(result.stderr, /does not match the qualified package/);
});
test('oversized regular archive is refused within the preparation budget', () => {
  const directory = fs.mkdtempSync(path.join(fixtures, 'oversize-'));
  const archive = path.join(directory, 'rom-ui-0.1.0-alpha.7.tgz');
  const descriptor = fs.openSync(archive, 'wx');
  try { fs.ftruncateSync(descriptor, 16 * 1024 * 1024 + 1); } finally { fs.closeSync(descriptor); }
  const result = prepare(archive);
  assert.equal(result.status, 1);
  assert.match(result.stderr, /exceeds the qualified file limit/);
});
