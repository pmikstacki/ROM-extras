import { test } from 'node:test';
import assert from 'node:assert/strict';
import { loadApprovedResourcePoints } from './host-points.ts';
const point = { id: 'Resource/00001:原', title: 'Approved', longitude: 180, latitude: -90 };
test('exact Resource ID and WGS84 edges survive a frozen snapshot', async () => {
  const rows = await loadApprovedResourcePoints(async () => [point], new AbortController().signal);
  assert.equal(rows[0].id, point.id);
  assert.ok(Object.isFrozen(rows) && Object.isFrozen(rows[0]));
});
test('duplicate identity, invalid coordinate and oversized snapshots fail', async () => {
  for (const rows of [[point, point], [{ ...point, longitude: 181 }], Array.from({ length: 201 }, (_, i) => ({ ...point, id: String(i) }))]) {
    await assert.rejects(loadApprovedResourcePoints(async () => rows, new AbortController().signal));
  }
});
test('cancelled authorization/read cannot publish its late snapshot', async () => {
  const controller = new AbortController();
  await assert.rejects(loadApprovedResourcePoints(async () => {
    controller.abort();
    return [point];
  }, controller.signal), { name: 'AbortError' });
});
