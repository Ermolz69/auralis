import assert from 'node:assert/strict';
import fs from 'node:fs';
import test from 'node:test';
import { checkPlan } from './plan.mjs';

const plan = () =>
  JSON.parse(
    fs.readFileSync(new URL('../../docs/integration/revoice-plan.json', import.meta.url), 'utf8'),
  );

test('the full endpoint cannot lose ASR, TXT, listening or clean Windows cases', () => {
  assert.deepEqual(checkPlan(plan()), []);
  for (const id of ['ASR-02', 'TXT-02', 'LISTEN-03', 'INSTALL-02']) {
    const candidate = plan();
    candidate.required_cases = candidate.required_cases.filter((c) => c.id !== id);
    assert.ok(checkPlan(candidate).some((e) => e.includes(`Missing required case ${id}`)));
  }
});

test('cycles, absent evidence and unfinished dependencies cannot become accepted work', () => {
  const candidate = plan();
  candidate.tasks[0].depends_on = [candidate.tasks[1].id];
  candidate.tasks[0].status = 'in_progress';
  candidate.tasks[1].status = 'done';
  candidate.tasks[1].evidence = [];
  assert.ok(checkPlan(candidate).some((e) => e.includes('Dependency cycle')));
  assert.ok(checkPlan(candidate).some((e) => e.includes('Missing acceptance evidence')));
  assert.ok(checkPlan(candidate).some((e) => e.includes('Unfinished dependency')));
});

test('a short native pass cannot complete the full Goal', () => {
  const candidate = plan();
  candidate.status = 'complete';
  candidate.required_cases[0].status = 'passed';
  candidate.required_cases[0].evidence = ['a-native-receipt.json'];
  assert.ok(checkPlan(candidate).some((e) => e.includes('Incomplete mandatory matrix')));
});
