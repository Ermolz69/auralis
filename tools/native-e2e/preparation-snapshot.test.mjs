import test from 'node:test';
import assert from 'node:assert/strict';
import { verifyEmptyPreparation } from './preparation-snapshot.mjs';

const SOURCE_SHA256 = 'a'.repeat(64);

function snapshot() {
  return {
    run: { state: 'paused', pause_requested: 0, run_id: 'run', translation_id: 'translation' },
    link: {
      active_run_id: 'run',
      translation_id: 'translation',
      selected_result_id: null,
      output_artifact_id: null,
      source_sha256: SOURCE_SHA256,
    },
    originalSha256: SOURCE_SHA256,
    counts: {
      jobs: 0,
      associations: 0,
      attempts: 0,
      checkpoints: 0,
      results: 0,
      publications: 0,
      outputArtifacts: 0,
    },
  };
}

test('accepts only the retained empty preparation boundary', () => {
  verifyEmptyPreparation(snapshot(), 'paused', SOURCE_SHA256);
});

test('rejects every kind of admitted or partial translation record', () => {
  for (const name of Object.keys(snapshot().counts)) {
    const value = snapshot();
    value.counts[name] = 1;
    assert.throws(() => verifyEmptyPreparation(value, 'paused', SOURCE_SHA256));
  }
});

test('rejects unfinished acknowledgement, another run and an exposed result', () => {
  for (const mutate of [
    (value) => {
      value.run.pause_requested = 1;
    },
    (value) => {
      value.run.state = 'running';
    },
    (value) => {
      value.link.active_run_id = 'another-run';
    },
    (value) => {
      value.link.translation_id = 'another-translation';
    },
    (value) => {
      value.link.selected_result_id = 'partial-result';
    },
    (value) => {
      value.link.output_artifact_id = 'partial-artifact';
    },
  ]) {
    const value = snapshot();
    mutate(value);
    assert.throws(() => verifyEmptyPreparation(value, 'paused', SOURCE_SHA256));
  }
});

test('rejects changed original bytes or source identity', () => {
  for (const mutate of [
    (value) => {
      value.originalSha256 = 'b'.repeat(64);
    },
    (value) => {
      value.link.source_sha256 = 'b'.repeat(64);
    },
  ]) {
    const value = snapshot();
    mutate(value);
    assert.throws(() => verifyEmptyPreparation(value, 'paused', SOURCE_SHA256));
  }
});
