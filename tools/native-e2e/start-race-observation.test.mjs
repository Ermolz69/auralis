import test from 'node:test';
import assert from 'node:assert/strict';
import { verifyConcurrentStarts } from './start-race-observation.mjs';

const RUN = 'retained-run';
const JOB = 'owned-job';
const records = () => [
  {
    boundary: 'initial_hash',
    runId: RUN,
    outcomes: Array.from({ length: 2 }, () => ({ status: 'rejected', code: 'BUSY' })),
  },
  {
    boundary: 'worker_preflight',
    runId: RUN,
    outcomes: Array.from({ length: 2 }, () => ({
      status: 'fulfilled',
      state: 'accepted',
      runId: RUN,
      jobId: JOB,
    })),
  },
];
const trace = (items) =>
  items.map((item) => `translation-concurrent-starts:${JSON.stringify(item)}\n`).join('');

test('requires both ordered parallel-start observations with exact retained identities', () => {
  assert.deepEqual(verifyConcurrentStarts(`other\n${trace(records())}`, RUN, JOB), records());
});

test('rejects missing, duplicated, reordered or misidentified boundary observations', () => {
  const valid = records();
  for (const items of [
    [],
    valid.slice(1),
    [...valid, valid[1]],
    valid.toReversed(),
    [{ ...valid[0], runId: 'other' }, valid[1]],
  ]) {
    assert.throws(() => verifyConcurrentStarts(trace(items), RUN, JOB));
  }
});

test('rejects a successful preparation start, wrong busy error or missing concurrent reply', () => {
  for (const outcomes of [
    [
      { status: 'fulfilled', state: 'accepted', runId: RUN, jobId: JOB },
      { status: 'rejected', code: 'BUSY' },
    ],
    [
      { status: 'rejected', code: 'CANCELLED' },
      { status: 'rejected', code: 'BUSY' },
    ],
    [{ status: 'rejected', code: 'BUSY' }],
  ]) {
    const valid = records();
    valid[0].outcomes = outcomes;
    assert.throws(() => verifyConcurrentStarts(trace(valid), RUN, JOB));
  }
});

test('rejects another job or run, busy instead of reuse, and nonaccepted replies', () => {
  for (const changed of [
    { jobId: 'other-job' },
    { runId: 'other-run' },
    { status: 'rejected', code: 'BUSY' },
    { state: 'running' },
    { jobId: undefined },
  ]) {
    const valid = records();
    valid[1].outcomes[0] = { ...valid[1].outcomes[0], ...changed };
    assert.throws(() => verifyConcurrentStarts(trace(valid), RUN, JOB));
  }
});
