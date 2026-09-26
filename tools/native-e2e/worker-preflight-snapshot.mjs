import assert from 'node:assert/strict';
import { PREPARATION_RECORD_KINDS, verifyPreparationIdentity } from './preparation-snapshot.mjs';

export function verifyWorkerPreflight(snapshot, sourceSha256, jobId, expectedStatus) {
  verifyPreparationIdentity(snapshot, 'paused', sourceSha256);
  assert.equal(snapshot.hostJobs.length, 1);
  const job = snapshot.hostJobs[0];
  assert.equal(job.id, jobId);
  assert.equal(job.project_id, snapshot.project.id);
  assert.equal(job.run_id, snapshot.run.run_id);
  assert.equal(job.kind, 'translation');
  assert.equal(job.status, expectedStatus);
  if (expectedStatus === 'running') assert.equal(job.ended_at, null);
  else {
    assert.equal(expectedStatus, 'cancelled');
    assert(job.ended_at, 'Cancelled preflight host association stayed open');
  }
  for (const name of PREPARATION_RECORD_KINDS) {
    const expected = name === 'jobs' || name === 'associations' ? 1 : 0;
    assert.equal(snapshot.counts[name], expected, `Worker preflight has unexpected ${name}`);
  }
}
