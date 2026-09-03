import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { load } from 'js-yaml';

const readText = (path) => readFileSync(new URL(`../../${path}`, import.meta.url), 'utf8');
const read = (path) => load(readText(path));
const ci = read('.github/workflows/ci.yml');
const bootstrap = read('.github/actions/bootstrap/action.yml');
const security = read('taskfiles/security.yml').tasks;

function assertSafeBrowserslist(lock) {
  const versions = Object.keys(lock.packages).filter((key) => key.startsWith('browserslist@'));
  assert.ok(versions.length > 0);
  for (const key of versions) {
    const version = key.slice('browserslist@'.length).split('.').map(Number);
    assert.ok(version.length === 3 && version.every(Number.isInteger));
    assert.ok(
      version[0] > 4 ||
        (version[0] === 4 && (version[1] > 28 || (version[1] === 28 && version[2] >= 7))),
      key,
    );
  }
}

test('the resolved Browserslist graph excludes both vulnerable advisory ranges', () => {
  assertSafeBrowserslist(read('pnpm-lock.yaml'));
  for (const version of ['4.28.4', '4.28.6']) {
    assert.throws(() => assertSafeBrowserslist({ packages: { [`browserslist@${version}`]: {} } }));
  }
  assertSafeBrowserslist({ packages: { 'browserslist@4.28.7': {} } });
  assert.equal(read('pnpm-workspace.yaml').overrides.browserslist, '4.28.7');
});

test('the full security gate audits both ecosystems and enforces Rust sources', () => {
  assert.deepEqual(security.audit.cmds, [
    { task: 'audit:npm' },
    { task: 'audit:rust' },
    { task: 'deny:rust' },
  ]);
  assert.deepEqual(security['audit:npm'].cmds, ['pnpm audit']);
  assert.deepEqual(security['audit:rust'].cmds, ['cargo audit --file Cargo.lock --deny unsound']);
  assert.deepEqual(security['deny:rust'].cmds, ['cargo deny --locked check advisories sources']);
  assert.deepEqual(read('Taskfile.yml').tasks['check:quality:security'].cmds, [
    { task: 'sec:audit' },
  ]);
});

test('local and CI installations pin identical Rust security tool versions', () => {
  const install = bootstrap.runs.steps.find((step) => step.uses === 'taiki-e/install-action@v2');
  assert.equal(install.if, "inputs.security == 'true'");
  assert.deepEqual(install.with.tool.split(','), ['cargo-audit@0.22.2', 'cargo-deny@0.20.2']);
  assert.deepEqual(security['setup:rust'].cmds, [
    'cargo install cargo-audit --version 0.22.2 --locked',
    'cargo install cargo-deny --version 0.20.2 --locked',
  ]);
});

test('security has a required independent job without GTK or browser bootstrap', () => {
  const job = ci.jobs.security;
  const setup = job.steps.find((step) => step.uses === './.github/actions/bootstrap');
  assert.deepEqual(setup.with, { node: 'true', security: 'true' });
  assert.ok(job.steps.some((step) => step.run === 'task check:quality:security'));
  assert.ok(ci.jobs['ci-summary'].needs.includes('security'));
  for (const group of ['frontend', 'rust', 'quality', 'release', 'ci', 'global']) {
    assert.ok(job.if.includes(`needs.changes.outputs.${group} == 'true'`));
  }
  const filters = load(ci.jobs.changes.steps.find((step) => step.id === 'filter').with.filters);
  assert.ok(filters.quality.includes('deny.toml'));
  assert.ok(filters.quality.includes('.cargo/**'));
  for (const name of ['frontend', 'docs', 'quality-global']) {
    const steps = ci.jobs[name].steps;
    assert.equal(
      steps.find((step) => step.uses === './.github/actions/bootstrap').with.security,
      undefined,
    );
    assert.ok(steps.every((step) => step.run !== 'task check:quality:security'));
  }
});

test('the Rust source policy rejects unknown origins without advisory suppressions', () => {
  const policy = readText('deny.toml');
  assert.match(policy, /unknown-registry\s*=\s*"deny"/);
  assert.match(policy, /unknown-git\s*=\s*"deny"/);
  assert.match(policy, /yanked\s*=\s*"deny"/);
  assert.match(policy, /unsound\s*=\s*"all"/);
  assert.doesNotMatch(policy, /^\s*ignore\s*=/m);
});

test('the Rust lockfile retains the compatible advisory and yank fixes', () => {
  const lock = readText('Cargo.lock');
  assert.match(lock, /name = "event-listener"\r?\nversion = "5\.4\.2"/);
  assert.match(lock, /name = "spin"\r?\nversion = "0\.9\.9"/);
});
