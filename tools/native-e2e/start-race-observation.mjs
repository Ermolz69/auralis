import assert from 'node:assert/strict';

const CHECKPOINT_PREFIX = 'translation-concurrent-starts:';
const CONCURRENT_START_COUNT = 2;

export function verifyConcurrentStarts(trace, runId, jobId) {
  const records = trace
    .split('\n')
    .filter((line) => line.startsWith(CHECKPOINT_PREFIX))
    .map((line) => JSON.parse(line.slice(CHECKPOINT_PREFIX.length)));
  assert.equal(records.length, 2, 'Concurrent-start observation is missing or duplicated');
  for (const [index, boundary] of ['initial_hash', 'worker_preflight'].entries()) {
    const record = records[index];
    assert.equal(record.boundary, boundary);
    assert.equal(record.runId, runId);
    assert.equal(record.outcomes.length, CONCURRENT_START_COUNT);
    for (const outcome of record.outcomes) {
      if (boundary === 'initial_hash') {
        assert.equal(outcome.status, 'rejected');
        assert.equal(outcome.code, 'BUSY');
      } else {
        assert.equal(outcome.status, 'fulfilled');
        assert.equal(outcome.state, 'accepted');
        assert.equal(outcome.jobId, jobId);
        assert.equal(outcome.runId, runId);
      }
    }
  }
  return records;
}
