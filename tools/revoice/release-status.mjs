import assert from 'node:assert/strict';
import fs from 'node:fs';
import { pathToFileURL } from 'node:url';

const states = new Set(['passed', 'failed', 'blocked', 'not_run']);
const sha = /^[a-f0-9]{64}$/;
const commit = /^[a-f0-9]{40}$/;
const required = Array.from({ length: 8 }, (_, i) => `RC-0${i + 1}`);

export function technicalStatus(record) {
  assert.equal(record.version, 1);
  assert.ok(states.has(record.status));
  assert.ok(states.has(record.specification_status));
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
  const values = [record.specification_status, ...record.tasks.map((task) => task.status)];
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
  if (values.includes('failed')) return 'failed';
  if (values.includes('blocked')) return 'blocked';
  if (values.includes('not_run')) return 'not_run';
  assert.match(record.candidate_commit ?? '', commit, 'Missing candidate commit');
  assert.match(record.candidate_executable_sha256 ?? '', sha, 'Missing candidate executable hash');
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
