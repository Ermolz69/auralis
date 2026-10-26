import assert from 'node:assert/strict';
import { test } from 'node:test';
import { installerBudget, runInstaller } from './installer.mjs';
test('expired and aborted installer budgets are rejected before spawn', async () => {
  assert.throws(
    () =>
      installerBudget(() => {
        throw new Error('expired');
      }),
    /expired/,
  );
  const abort = new AbortController();
  abort.abort(new Error('cancelled before spawn'));
  await assert.rejects(
    runInstaller('must-not-execute', [], {}, () => 1000, abort.signal),
    /cancelled before spawn/,
  );
});
