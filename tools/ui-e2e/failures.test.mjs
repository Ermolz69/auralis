import assert from 'node:assert/strict';
import { test } from 'node:test';
import { uiFailure } from './failures.mjs';
import { Blocked } from './core.mjs';
test('only identified readiness or external-source failures are blockers', () => {
  for (const message of [
    'HTTP Error 403: Forbidden',
    'TTS runtime is not configured',
    'Video unavailable',
  ])
    assert.ok(uiFailure(message) instanceof Blocked);
  for (const message of [
    'Operation not supported',
    'An unexpected system error occurred',
    'Downloaded model digest mismatch',
    'Fixture integrity verification failed',
  ])
    assert.equal(uiFailure(message) instanceof Blocked, false);
});
