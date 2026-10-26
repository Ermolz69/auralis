import assert from 'node:assert/strict';
import fs from 'node:fs';
import { createHash } from 'node:crypto';
import { pathToFileURL } from 'node:url';
import { verifyEvidence } from './release-evidence.mjs';

const states = new Set(['passed', 'failed', 'blocked', 'not_run']);
const sha = /^[a-f0-9]{64}$/;
const commit = /^[a-f0-9]{40}$/;
const required = Array.from({ length: 8 }, (_, i) => `RC-0${i + 1}`);
const requiredDefects = Array.from(
  { length: 13 },
  (_, i) => `RC-D${String(i + 1).padStart(2, '0')}`,
);

export function technicalStatus(record, readEvidence) {
  assert.equal(record.version, 1);
  assert.ok(states.has(record.status));
  assert.ok(states.has(record.specification_status));
  assert.equal(record.source.url, 'https://www.youtube.com/watch?v=HRaa8kb1EhE');
  assert.equal(record.source.expected_cues, 4004, 'Full source coverage changed');
  assert.equal(record.source.subtitle_bytes, 236229);
  assert.equal(
    record.source.subtitle_sha256,
    '868dc24073183cfd9e5f41ded3eb0cca34b6faad53e008a8b50ddb6f79c49735',
  );
  assert.deepEqual(record.source.track, { language: 'zh', format: 'srt', automatic: false });
  assert.match(record.translate_commit ?? '', commit);
  assert.ok(states.has(record.source.video_duration_verification));
  const runtime = record.prospective_runtime;
  assert.ok(runtime && states.has(runtime.status));
  assert.match(runtime.seed_manifest_sha256 ?? '', sha);
  for (const key of ['contract', 'tts']) assert.match(runtime.component_commits[key] ?? '', commit);
  assert.equal(runtime.wheels.length, 2);
  assert.deepEqual(runtime.wheels.map((wheel) => wheel.name).sort(), [
    'puza_tts',
    'voxscript_contract',
  ]);
  for (const wheel of runtime.wheels) assert.match(wheel.sha256 ?? '', sha);
  for (const wheel of runtime.wheels) assert.match(wheel.version ?? '', /^\d+\.\d+\.\d+$/);
  assert.match(runtime.translation.sha256 ?? '', sha);
  assert.match(runtime.translation.runtime_sha256 ?? '', sha);
  assert.deepEqual(
    runtime.engines.map((item) => item.engine_id),
    ['voxcpm2', 'espeech'],
  );
  for (const engine of runtime.engines) {
    assert.match(engine.model_revision ?? '', commit);
    for (const key of ['profile_sha256', 'voice_manifest_sha256', 'reference_sha256'])
      assert.match(engine[key] ?? '', sha);
    for (const key of ['profile_id', 'voice_id', 'reference_id']) assert.ok(engine[key]);
  }
  assert.ok(Array.isArray(record.tasks));
  const ids = record.tasks.map((task) => task.id);
  assert.equal(new Set(ids).size, ids.length, 'Duplicate task');
  assert.deepEqual([...ids].sort(), required, 'Required task set changed');
  for (const task of record.tasks) {
    assert.ok(task.owner && states.has(task.status));
    assert.ok(Array.isArray(task.depends_on));
    for (const dependency of task.depends_on) {
      assert.ok(ids.includes(dependency) && dependency !== task.id, 'Invalid dependency');
      if (task.status === 'passed')
        assert.equal(record.tasks.find((item) => item.id === dependency).status, 'passed');
    }
  }
  const visiting = new Set();
  const visited = new Set();
  function visit(id) {
    assert.ok(!visiting.has(id), 'Cyclic dependency');
    if (visited.has(id)) return;
    visiting.add(id);
    for (const dependency of record.tasks.find((task) => task.id === id).depends_on)
      visit(dependency);
    visiting.delete(id);
    visited.add(id);
  }
  for (const id of ids) visit(id);
  assert.deepEqual(
    record.engine_results.map((item) => item.engine_id),
    ['voxcpm2', 'espeech'],
  );
  const defectIds = record.defects.map((item) => item.id);
  assert.equal(new Set(defectIds).size, defectIds.length, 'Duplicate defect');
  assert.ok(
    requiredDefects.every((id) => defectIds.includes(id)),
    'Required defect set changed',
  );
  const values = [
    record.specification_status,
    record.source.video_duration_verification,
    runtime.status,
    ...record.tasks.map((task) => task.status),
  ];
  for (const defect of record.defects) {
    assert.ok(defect.owner && states.has(defect.status));
    values.push(defect.status);
  }
  for (const engine of record.engine_results)
    for (const key of ['diagnostic', 'full_source', 'restart_playback', 'process_release']) {
      assert.ok(states.has(engine[key]));
      values.push(engine[key]);
    }
  assert.equal(record.broader_release_matrix_changed, false);
  assert.equal(record.broader_release_matrix, 'docs/integration/revoice-plan.json');
  const matrixHash = createHash('sha256')
    .update(fs.readFileSync(record.broader_release_matrix))
    .digest('hex');
  assert.equal(matrixHash, record.broader_release_matrix_sha256, 'Broader release matrix changed');
  if (values.includes('failed')) return 'failed';
  if (values.includes('blocked')) return 'blocked';
  if (values.includes('not_run')) return 'not_run';
  assert.match(record.candidate_commit ?? '', commit, 'Missing candidate commit');
  assert.match(record.candidate_executable_sha256 ?? '', sha, 'Missing candidate executable hash');
  verifyEvidence(record, readEvidence);
  return 'passed';
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const record = JSON.parse(
    fs.readFileSync('docs/integration/revoice-release-candidate.json', 'utf8'),
  );
  const status = technicalStatus(record);
  assert.equal(record.status, status, 'Stored status differs from mandatory stage results');
  process.stdout.write(`Technical candidate: ${status}\nBroader release gates: unchanged\n`);
  process.exitCode = status === 'passed' ? 0 : status === 'failed' ? 1 : 2;
}
