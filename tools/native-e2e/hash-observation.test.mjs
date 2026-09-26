import test from 'node:test';
import assert from 'node:assert/strict';
import {
  observedOpenHash,
  interruptedHash,
  parseHashRecords,
  verifyFreshHashes,
} from './hash-observation.mjs';

const started = () => ({
  event_name: 'translation_model_hash_started',
  hash_operation: 1,
  hashed_bytes: 65_536,
  total_bytes: 1_133_080_448,
});
const failed = () => ({
  ...started(),
  event_name: 'translation_model_hash_finished',
  outcome: 'failed',
  hashed_bytes: 262_144,
});
const fresh = () => ({ ...started(), hash_operation: 2 });
const completed = () => ({
  ...fresh(),
  event_name: 'translation_model_hash_finished',
  outcome: 'completed',
  hashed_bytes: started().total_bytes,
});

test('observes actual partial hash, failed completion and a distinct fresh complete check', () => {
  assert.deepEqual(observedOpenHash([started()], started().total_bytes), started());
  assert.deepEqual(interruptedHash([started(), failed()], started()), failed());
  assert.equal(observedOpenHash([started(), failed()], started().total_bytes), null);
  assert.equal(
    verifyFreshHashes([started(), failed(), fresh(), completed()], { started: started() }).length,
    1,
  );
});

test('rejects absent, invalid, fully read or ambiguous open counters', () => {
  for (const field of ['hash_operation', 'hashed_bytes', 'total_bytes']) {
    for (const value of [undefined, -1, 0, '1', Number.MAX_SAFE_INTEGER + 1]) {
      const record = { ...started(), [field]: value };
      if (field === 'total_bytes')
        assert.equal(observedOpenHash([record], started().total_bytes), null);
      else assert.throws(() => observedOpenHash([record], started().total_bytes));
    }
  }
  assert.throws(() =>
    observedOpenHash(
      [{ ...started(), hashed_bytes: started().total_bytes }],
      started().total_bytes,
    ),
  );
  assert.throws(() => observedOpenHash([started(), fresh()], started().total_bytes));
});

test('rejects successful, complete, regressing, mismatched or duplicated interruption', () => {
  for (const finish of [
    { ...failed(), outcome: 'completed' },
    { ...failed(), hashed_bytes: started().total_bytes },
    { ...failed(), hashed_bytes: 1 },
    { ...failed(), total_bytes: 1_000_000 },
    { ...failed(), hashed_bytes: undefined },
  ])
    assert.throws(() => interruptedHash([started(), finish], started()));
  assert.equal(interruptedHash([{ ...failed(), hash_operation: 2 }], started()), null);
  assert.throws(() => interruptedHash([failed(), failed()], started()));
});

test('rejects resume without a new completed operation or with partial duplicate results', () => {
  const interruption = { started: started() };
  for (const records of [
    [started(), failed()],
    [started(), failed(), fresh()],
    [started(), failed(), fresh(), { ...completed(), outcome: 'failed' }],
    [started(), failed(), fresh(), { ...completed(), hashed_bytes: 1 }],
    [started(), failed(), fresh(), completed(), completed()],
    [started(), failed(), fresh(), fresh(), completed()],
  ])
    assert.throws(() => verifyFreshHashes(records, interruption));
});

test('reads only complete JSON log records and filters unrelated diagnostics', () => {
  const text = `${JSON.stringify({ fields: started(), timestamp: 'start' })}\n${JSON.stringify({ fields: { event_name: 'other' } })}\n{"unfinished":`;
  assert.deepEqual(parseHashRecords(text), [{ ...started(), timestamp: 'start' }]);
  assert.throws(() => parseHashRecords('invalid complete line\n'));
});
