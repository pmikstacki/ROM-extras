import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { FixtureProcesses } from './map-fixture-processes.mjs';

const root = fileURLToPath(new URL('../', import.meta.url));
const tls = process.env.ROM_EXTRAS_MAP_FIXTURE_TLS;
if (!tls) throw Error('Explicit controlled fixture TLS directory required');
const frontend = path.join(root, 'examples/maps-ui');
const data = path.join(root, '.superpowers/maps-ui-browser-fixtures');
fs.mkdirSync(data, { recursive: true });
const store = path.join(fs.mkdtempSync(path.join(data, 'run-')), 'demo.sqlite');
const env = { ...process.env, ROM_LIVE_FIXTURE: '1' };
const scope = new FixtureProcesses();
const interrupted = () => { scope.close().catch(() => {}).finally(() => process.exit(1)); };
process.once('SIGINT', interrupted);
process.once('SIGTERM', interrupted);
let stage = 'provider readiness';
try {
  const provider = scope.start(process.execPath, ['crates/rom-maptiler/tests/receiver.mjs'], { cwd: root, env });
  const nativePort = await scope.readyLine(provider);
  if (!/^[1-9][0-9]{0,4}$/.test(nativePort) || Number(nativePort) > 65535) throw Error('Controlled fixture invalid port');
  stage = 'host readiness';
  const host = scope.start(path.join(root, 'target/maps-live-host/debug/examples/demo'), [
    `https://127.0.0.1:${nativePort}/operator/`, path.join(tls, 'ca.pem'), store,
  ], { cwd: root, env });
  if (await scope.readyLine(host) !== 'Controlled local ROM host ready') throw Error('Controlled fixture invalid host readiness');
  stage = 'browser checks';
  const browser = scope.start('corepack', ['pnpm', 'exec', 'playwright', 'test'], { cwd: frontend, env, stdio: 'inherit' });
  await scope.wait(browser);
  console.log(`Controlled browser checks passed; fixture database retained at ${store}`);
} catch {
  console.error(`Controlled map browser fixture failed during ${stage}`);
  process.exitCode = 1;
} finally {
  await scope.close();
  process.off('SIGINT', interrupted);
  process.off('SIGTERM', interrupted);
}
