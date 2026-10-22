import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export const requiredCases = Object.freeze([
  'SUB-01',
  'SUB-02',
  'SUB-03',
  'TTS-NATIVE',
  'LONG-01',
  'LONG-02',
  'FIT-01',
  'FIT-02',
  'FIT-03',
  'EXPORT-01',
  'EXPORT-02',
  'TXT-01',
  'TXT-02',
  'ASR-01',
  'ASR-02',
  'LANG-01',
  'REC-01',
  'LISTEN-01',
  'LISTEN-02',
  'LISTEN-03',
  'VOICES-192',
  'INSTALL-01',
  'INSTALL-02',
  'AUDIT-01',
]);

export function checkPlan(plan) {
  const errors = [];
  const tasks = new Map();
  for (const task of plan.tasks ?? []) {
    if (!/^INT-R\d{2}$/.test(task.id) || tasks.has(task.id))
      errors.push(`Invalid/repeated task ${task.id}`);
    tasks.set(task.id, task);
    if (!['planned', 'ready', 'in_progress', 'blocked', 'done'].includes(task.status))
      errors.push(`Invalid state ${task.id}`);
    if (!task.bindings?.length) errors.push(`Missing component binding ${task.id}`);
    if (task.status === 'blocked' && !task.blocker?.trim())
      errors.push(`Missing blocker ${task.id}`);
    if (task.status === 'done' && !task.evidence?.length)
      errors.push(`Missing acceptance evidence ${task.id}`);
  }
  const visiting = new Set();
  const visited = new Set();
  function visit(id) {
    if (visiting.has(id)) {
      errors.push(`Dependency cycle ${id}`);
      return;
    }
    if (visited.has(id)) return;
    visiting.add(id);
    const task = tasks.get(id);
    for (const dependency of task?.depends_on ?? []) {
      if (!tasks.has(dependency)) errors.push(`Unknown dependency ${id} -> ${dependency}`);
      else {
        if (['ready', 'done'].includes(task.status) && tasks.get(dependency).status !== 'done')
          errors.push(`Unfinished dependency ${id} -> ${dependency}`);
        visit(dependency);
      }
    }
    visiting.delete(id);
    visited.add(id);
  }
  for (const id of tasks.keys()) visit(id);
  const cases = new Map();
  for (const item of plan.required_cases ?? []) {
    if (cases.has(item.id)) errors.push(`Repeated case ${item.id}`);
    cases.set(item.id, item);
    if (!['pending', 'blocked', 'failed', 'passed'].includes(item.status))
      errors.push(`Invalid case state ${item.id}`);
    if (item.status === 'passed' && !item.evidence?.length)
      errors.push(`Missing case evidence ${item.id}`);
  }
  for (const id of requiredCases) if (!cases.has(id)) errors.push(`Missing required case ${id}`);
  if (
    plan.status === 'complete' &&
    ([...tasks.values()].some((t) => t.status !== 'done') ||
      [...cases.values()].some((c) => c.status !== 'passed'))
  )
    errors.push('Incomplete mandatory matrix cannot complete the Goal');
  return errors;
}

const currentFile = fileURLToPath(import.meta.url);
if (path.resolve(process.argv[1] ?? '') === currentFile) {
  const root = path.resolve(path.dirname(currentFile), '../..');
  const plan = JSON.parse(
    fs.readFileSync(path.join(root, 'docs/integration/revoice-plan.json'), 'utf8'),
  );
  const errors = checkPlan(plan);
  if (errors.length) {
    process.stderr.write(`${errors.join('\n')}\n`);
    process.exitCode = 1;
  } else
    process.stdout.write(
      `Revoicing plan valid: ${plan.tasks.length} tasks, ${plan.required_cases.length} mandatory cases; status ${plan.status}.\n`,
    );
}
