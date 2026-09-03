import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { load } from 'js-yaml';

const root = new URL('../../', import.meta.url);
const frontend = 'pnpm install --frozen-lockfile';
const rust = 'cargo fetch --locked';

function commands(task) {
  const result = spawnSync('task', ['--dry', task], {
    cwd: fileURLToPath(root),
    encoding: 'utf8',
    timeout: 15000,
    env: { ...process.env, FORCE_COLOR: '0', NO_COLOR: '1' },
  });
  assert.ifError(result.error);
  assert.equal(result.status, 0, result.stderr);
  return `${result.stdout}\n${result.stderr}`
    .split(/\r?\n/)
    .filter((line) => line.startsWith('task: ['))
    .map((line) => line.replace(/^task: \[[^\]]+\] /, ''));
}

test('frontend installation never invokes Rust or browser setup', () => {
  assert.deepEqual(commands('install:frontend'), [frontend]);
});

test('Rust installation never invokes Node, pnpm or browser setup', () => {
  assert.deepEqual(commands('install:rust'), [rust]);
});

test('full installation composes both scoped installers exactly once', () => {
  assert.deepEqual(commands('install:all'), [frontend, rust]);
});

test('legacy install remains a compatible alias for the full installation', () => {
  assert.deepEqual(commands('install'), [frontend, rust]);
});

test('the CI entrypoint explicitly installs both components before checks', () => {
  const taskfile = load(readFileSync(new URL('Taskfile.yml', root), 'utf8'));
  assert.deepEqual(taskfile.tasks.ci.cmds, [{ task: 'install:all' }, { task: 'check' }]);
  assert.deepEqual(commands('ci').slice(0, 2), [frontend, rust]);
});

test('README setup installs dependencies and Chromium before the first check command', () => {
  const readme = readFileSync(new URL('README.md', root), 'utf8');
  const install = readme.indexOf('task install:all');
  const browser = readme.indexOf('task fe:setup:playwright');
  const check = readme.indexOf('task check');
  assert.ok(install >= 0 && browser > install && check > browser);
  assert.ok(readme.includes('task fe:setup:playwright:ci'));
});
