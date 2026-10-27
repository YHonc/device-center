import test from 'node:test';
import assert from 'node:assert/strict';
import { createRequestGate } from '../src/requestGate.js';

test('late read from the previous device cannot replace the new snapshot', async () => {
  const gate = createRequestGate();
  let resolveOld;
  let snapshot;
  const oldRead = new Promise(resolve => { resolveOld = resolve; });
  const oldCurrent = gate.begin();
  const pending = oldRead.then(value => { if (oldCurrent()) snapshot = value; });
  const newCurrent = gate.begin();
  if (newCurrent()) snapshot = 'device B';
  resolveOld('device A');
  await pending;
  assert.equal(snapshot, 'device B');
});

test('closing the settings scope invalidates an in-flight result', () => {
  const gate = createRequestGate();
  const current = gate.begin();
  gate.invalidate();
  assert.equal(current(), false);
  assert.equal(gate.begin()(), true);
});
