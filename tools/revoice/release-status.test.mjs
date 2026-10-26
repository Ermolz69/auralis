import assert from 'node:assert/strict';
import fs from 'node:fs';
import { test } from 'node:test';
import { technicalStatus } from './release-status.mjs';

const base = JSON.parse(fs.readFileSync('docs/integration/revoice-release-candidate.json', 'utf8'));
function complete() {
  const record = structuredClone(base);
  record.status = record.specification_status = 'passed';
  record.candidate_commit = 'a'.repeat(40);
  record.candidate_executable_sha256 = 'b'.repeat(64);
  for (const task of record.tasks) task.status = 'passed';
  for (const defect of record.defects) defect.status = 'passed';
  for (const engine of record.engine_results)
    for (const key of ['diagnostic', 'full_source', 'restart_playback', 'process_release'])
      engine[key] = 'passed';
  return record;
}

test('required unrun or blocked engine work cannot pass', () => {
  assert.equal(technicalStatus(complete()), 'passed');
  for (const status of ['failed', 'blocked', 'not_run']) {
    const record = complete();
    record.engine_results[1].full_source = status;
    assert.equal(technicalStatus(record), status);
  }
});
test('missing coverage, candidate identity or satisfied dependency is rejected', () => {
  const missing = complete();
  missing.tasks.pop();
  assert.throws(() => technicalStatus(missing), /Required task set/);
  const noHash = complete();
  noHash.candidate_executable_sha256 = null;
  assert.throws(() => technicalStatus(noHash), /Missing candidate executable/);
  const dependency = complete();
  dependency.tasks[0].status = 'blocked';
  assert.throws(() => technicalStatus(dependency));
  const cyclic = complete();
  cyclic.tasks[0].depends_on = ['RC-08'];
  assert.throws(() => technicalStatus(cyclic), /Cyclic dependency/);
});
test('technical scope cannot silently alter broader gates', () => {
  const record = complete();
  record.broader_release_matrix_changed = true;
  assert.throws(() => technicalStatus(record));
});
