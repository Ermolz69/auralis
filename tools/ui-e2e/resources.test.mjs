import assert from 'node:assert/strict';
import { test } from 'node:test';
import { enforceResources, ResourceObserver } from './resources.mjs';
import { observeUntil, setObservationSignal } from './observe.mjs';
import fsp from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { Blocked } from './core.mjs';
const budget = {
  max_rss_bytes: 100,
  max_vram_bytes: 80,
  baseline_vram_bytes: 20,
  max_profile_bytes: 200,
  min_free_disk_bytes: 40,
};
const sample = { rss_bytes: 90, vram_bytes: 10, profile_bytes: 100, free_disk_bytes: 50 };
test('resource overruns and unavailable observations cannot authorize inference', () => {
  enforceResources(sample, budget, true);
  for (const change of [
    { rss_bytes: 101 },
    { vram_bytes: 81 },
    { profile_bytes: 201 },
    { free_disk_bytes: 39 },
  ])
    assert.throws(() => enforceResources({ ...sample, ...change }, budget), Blocked);
  assert.throws(() => enforceResources({ ...sample, vram_bytes: 21 }, budget, true), Blocked);
  assert.throws(() => enforceResources({ ...sample, rss_bytes: NaN }, budget), /missing/);
});

test('an unwritable resource log still stops work and interrupts a pending UI read', async () => {
  const directory = await fsp.mkdtemp(path.join(os.tmpdir(), 'auralis-ui-observer-'));
  const invalid = path.join(directory, 'regular-file');
  await fsp.writeFile(invalid, 'fixture');
  const abort = new AbortController();
  let stopped = 0;
  setObservationSignal(abort.signal);
  const wait = observeUntil(() => new Promise(() => {}), 60000);
  const failedWait = assert.rejects(wait, (error) => ['ENOTDIR', 'ENOENT'].includes(error.code));
  const observer = new ResourceObserver(
    { directory: invalid },
    budget,
    async () => [],
    (error) => {
      stopped++;
      abort.abort(error);
    },
  );
  try {
    await assert.rejects(observer.auditProfile(), (error) =>
      ['ENOTDIR', 'ENOENT'].includes(error.code),
    );
    await failedWait;
    assert.equal(stopped, 1);
    assert.ok(['ENOTDIR', 'ENOENT'].includes(observer.failure.code));
    assert.ok((await observer.stop()).retention_error);
  } finally {
    setObservationSignal(undefined);
    await fsp.unlink(invalid);
    await fsp.rmdir(directory);
  }
});
