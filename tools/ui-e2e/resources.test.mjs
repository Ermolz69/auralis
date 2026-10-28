import assert from 'node:assert/strict';
import { test } from 'node:test';
import { enforceResources } from './resources.mjs';
import { Blocked } from './core.mjs';
const budget = { max_rss_bytes: 100, max_vram_bytes: 80, baseline_vram_bytes: 20, max_profile_bytes: 200, min_free_disk_bytes: 40 };
const sample = { rss_bytes: 90, vram_bytes: 10, profile_bytes: 100, free_disk_bytes: 50 };
test('resource overruns and unavailable observations cannot authorize inference', () => {
  enforceResources(sample, budget, true);
  for (const change of [{rss_bytes:101},{vram_bytes:81},{profile_bytes:201},{free_disk_bytes:39}])
    assert.throws(() => enforceResources({...sample,...change}, budget), Blocked);
  assert.throws(() => enforceResources({...sample,vram_bytes:21}, budget,true), Blocked);
  assert.throws(() => enforceResources({...sample,rss_bytes:NaN}, budget), /missing/);
});
