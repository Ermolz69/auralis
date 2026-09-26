import test from 'node:test';
import assert from 'node:assert/strict';
import { verifyWorkerPreflight } from './worker-preflight-snapshot.mjs';

const SOURCE_SHA256 = 'a'.repeat(64);

function snapshot(status = 'running') {
  return {
    project: { id: 'project' },
    run: { state: 'paused', pause_requested: 0, run_id: 'run', translation_id: 'translation' },
    link: {
      active_run_id: 'run',
      translation_id: 'translation',
      selected_result_id: null,
      output_artifact_id: null,
      source_sha256: SOURCE_SHA256,
    },
    originalSha256: SOURCE_SHA256,
    hostJobs: [
      {
        id: 'job',
        project_id: 'project',
        run_id: 'run',
        kind: 'translation',
        status,
        ended_at: status === 'running' ? null : 'closed',
      },
    ],
    counts: {
      jobs: 1,
      associations: 1,
      attempts: 0,
      checkpoints: 0,
      results: 0,
      publications: 0,
      outputArtifacts: 0,
    },
  };
}

test('accepts active and cancelled host preflight without a Translate attempt', () => {
  for (const status of ['running', 'cancelled']) {
    verifyWorkerPreflight(snapshot(status), SOURCE_SHA256, 'job', status);
  }
});

test('rejects every nonmatching admitted or partial record count', () => {
  for (const name of Object.keys(snapshot().counts)) {
    const value = snapshot();
    value.counts[name] += 1;
    assert.throws(() => verifyWorkerPreflight(value, SOURCE_SHA256, 'job', 'running'));
  }
});

test('rejects omitted required record counts', () => {
  for (const name of Object.keys(snapshot().counts)) {
    const value = snapshot();
    delete value.counts[name];
    assert.throws(() => verifyWorkerPreflight(value, SOURCE_SHA256, 'job', 'running'));
  }
});

test('rejects another job, project, run or kind', () => {
  for (const field of ['id', 'project_id', 'run_id', 'kind']) {
    const value = snapshot();
    value.hostJobs[0][field] = 'another';
    assert.throws(() => verifyWorkerPreflight(value, SOURCE_SHA256, 'job', 'running'));
  }
});

test('rejects absent or duplicate host associations', () => {
  for (const jobs of [[], [...snapshot().hostJobs, ...snapshot().hostJobs]]) {
    const value = snapshot();
    value.hostJobs = jobs;
    assert.throws(() => verifyWorkerPreflight(value, SOURCE_SHA256, 'job', 'running'));
  }
});

test('rejects wrong host state and inverted terminal markers', () => {
  for (const status of ['running', 'cancelled']) {
    const value = snapshot(status);
    value.hostJobs[0].ended_at = status === 'running' ? 'closed' : null;
    assert.throws(() => verifyWorkerPreflight(value, SOURCE_SHA256, 'job', status));
    value.hostJobs[0].status = 'completed';
    assert.throws(() => verifyWorkerPreflight(value, SOURCE_SHA256, 'job', status));
  }
});
