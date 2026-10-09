import test from 'node:test';
import assert from 'node:assert/strict';
import net from 'node:net';
import { fileURLToPath } from 'node:url';
import { FixtureProcesses } from '../../scripts/map-fixture-processes.mjs';

function listens(port) {
  return new Promise(resolve => {
    const socket = net.connect({host:'127.0.0.1',port});
    const finish = value => { socket.destroy(); resolve(value); };
    socket.once('connect', () => finish(true));
    socket.once('error', () => finish(false));
    socket.setTimeout(500, () => finish(false));
  });
}
test('closing owned parent also closes its actual descendant listener', async () => {
  const scope = new FixtureProcesses();
  let descendant;
  try {
    const parent = scope.start(process.execPath, [fileURLToPath(new URL('./process-parent.mjs', import.meta.url))]);
    descendant = JSON.parse(await scope.readyLine(parent));
    assert.ok(Number.isInteger(descendant.pid) && descendant.pid > 1 && descendant.pid !== process.pid && descendant.pid !== parent.pid);
    assert.equal(await listens(descendant.port), true);
    await scope.close();
    assert.equal(await listens(descendant.port), false);
  } finally {
    await scope.close();
    if (descendant) {
      try { process.kill(descendant.pid, 'SIGKILL'); } catch (error) { if (error.code !== 'ESRCH') throw error; }
    }
  }
});
test('silent readiness and active process waits remain bounded', async () => {
  const scope = new FixtureProcesses();
  try {
    const child = scope.start(process.execPath, ['-e', 'setInterval(()=>{},1000)']);
    await assert.rejects(scope.readyLine(child, 100), /readiness deadline/);
    await assert.rejects(scope.wait(child, 100), /process deadline/);
  } finally { await scope.close(); }
});
test('missing executable fails without an unhandled spawn error', async () => {
  const scope = new FixtureProcesses();
  try {
    const child = scope.start('/rom-controlled-fixture-executable-does-not-exist', []);
    await assert.rejects(scope.readyLine(child), /closed before readiness/);
    await assert.rejects(scope.wait(child), /process failed/);
  } finally { await scope.close(); }
});
test('oversized readiness is rejected before a fixture can be admitted', async () => {
  const scope = new FixtureProcesses();
  try {
    const child = scope.start(process.execPath, ['-e', 'console.log("x".repeat(1025));setInterval(()=>{},1000)']);
    await assert.rejects(scope.readyLine(child), /exceeded limit/);
  } finally { await scope.close(); }
});
