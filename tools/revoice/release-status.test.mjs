import assert from 'node:assert/strict';
import fs from 'node:fs';
import { test } from 'node:test';
import { createHash } from 'node:crypto';
import { technicalStatus } from './release-status.mjs';

const base = JSON.parse(fs.readFileSync('docs/integration/revoice-release-candidate.json', 'utf8'));
function reportBytes(id) {
  return JSON.stringify({
    version: 1,
    task_id: id,
    status: 'passed',
    candidate_sha256: 'b'.repeat(64),
    source_subtitle_sha256: base.source.subtitle_sha256,
    runtime_seed_sha256: base.prospective_runtime.seed_manifest_sha256,
    source_cue_count: 4004,
    case_scope: id === 'RC-06' ? 'diagnostic' : 'full-source',
    case_id: `TEST-${id}`,
    engine_ids: ['voxcpm2', 'espeech'],
  });
}
const readFixture = (file) => reportBytes(file.split('/').at(-1).replace('.json', ''));
const assess = (record) => technicalStatus(record, readFixture);
function complete() {
  const record = structuredClone(base);
  record.status = record.specification_status = 'passed';
  record.candidate_commit = 'a'.repeat(40);
  record.candidate_executable_sha256 = 'b'.repeat(64);
  record.source.video_duration_verification = record.prospective_runtime.status = 'passed';
  for (const task of record.tasks) {
    task.status = 'passed';
    task.evidence = [
      {
        path: `tools/revoice/evidence/${task.id}.json`,
        sha256: createHash('sha256').update(reportBytes(task.id)).digest('hex'),
      },
    ];
  }
  for (const defect of record.defects) defect.status = 'passed';
  for (const engine of record.engine_results)
    for (const key of ['diagnostic', 'full_source', 'restart_playback', 'process_release'])
      engine[key] = 'passed';
  return record;
}

test('required unrun or blocked engine work cannot pass', () => {
  assert.equal(assess(complete()), 'passed');
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
  const changedHash = complete();
  changedHash.broader_release_matrix_sha256 = '0'.repeat(64);
  assert.throws(() => technicalStatus(changedHash), /Broader release matrix changed/);
});
test('the actual stored register agrees with its mandatory aggregate', () => {
  assert.equal(base.status, technicalStatus(base));
});
test('source/runtime prerequisites and the complete defect set cannot disappear', () => {
  for (const status of ['failed', 'blocked', 'not_run']) {
    for (const key of ['source', 'prospective_runtime']) {
      const record = complete();
      record[key][key === 'source' ? 'video_duration_verification' : 'status'] = status;
      assert.equal(technicalStatus(record), status);
    }
  }
  const removed = complete();
  removed.defects = [];
  assert.throws(() => technicalStatus(removed), /Required defect set changed/);
  const removedLatest = complete();
  removedLatest.defects.pop();
  assert.throws(() => technicalStatus(removedLatest), /Required defect set changed/);
  const reduced = complete();
  reduced.source.expected_cues = 10;
  assert.throws(() => technicalStatus(reduced), /Full source coverage changed/);
  const missingPin = complete();
  delete missingPin.prospective_runtime.engines[1].reference_sha256;
  assert.throws(() => technicalStatus(missingPin));
  const wrongCandidate = complete();
  wrongCandidate.candidate_executable_sha256 = '0'.repeat(64);
  assert.throws(() => assess(wrongCandidate));
  const noComponent = complete();
  noComponent.prospective_runtime.component_commits = {};
  assert.throws(() => assess(noComponent));
  const duplicate = complete();
  duplicate.prospective_runtime.wheels[1] = duplicate.prospective_runtime.wheels[0];
  assert.throws(() => assess(duplicate));
});
test('failed, changed or diagnostic-only reports cannot establish complete source acceptance', () => {
  for (const mutation of [
    (report) => {
      report.status = 'failed';
    },
    (report) => {
      report.source_subtitle_sha256 = '0'.repeat(64);
    },
    (report) => {
      report.case_scope = 'diagnostic';
    },
  ]) {
    const record = complete();
    const report = JSON.parse(reportBytes('RC-07'));
    mutation(report);
    const bytes = JSON.stringify(report);
    record.tasks[6].evidence[0].sha256 = createHash('sha256').update(bytes).digest('hex');
    assert.throws(() =>
      technicalStatus(record, (file) => (file.endsWith('RC-07.json') ? bytes : readFixture(file))),
    );
  }
  assert.throws(() => technicalStatus(complete(), () => '{}'));
});
