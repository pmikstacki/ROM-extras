import { test } from 'node:test';
import assert from 'node:assert/strict';
import { requestSelection } from './host-selection.ts';
const generation = 'epoch1';
test('exact host outcomes and identity pass through', async () => {
  for (const outcome of ['accepted', 'rejected', 'unknown'] as const) {
    assert.equal(await requestSelection('ę/0001:Straße', generation, () => generation, async (body) => {
      assert.deepEqual(body, { id: 'ę/0001:Straße', generation });
      return { generation, outcome };
    }), outcome);
  }
});
test('network loss and malformed outcomes remain unknown', async () => {
  for (const send of [async () => { throw new Error('Lost reply'); }, async () => ({ generation, outcome: 'invalid' }), async () => ({ generation: 'other', outcome: 'accepted' })]) {
    assert.equal(await requestSelection('id', generation, () => generation, send), 'unknown');
  }
});
test('stale session before dispatch refuses without sending; after dispatch remains unknown', async () => {
  assert.equal(await requestSelection('id', generation, () => 'other', async () => { throw new Error('Must not dispatch'); }), 'rejected');
  let current = generation;
  assert.equal(await requestSelection('id', generation, () => current, async () => { current = 'other'; return { generation, outcome: 'accepted' }; }), 'unknown');
});
