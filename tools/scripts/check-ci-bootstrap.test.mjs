import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { load } from 'js-yaml';

const read = (path) => load(readFileSync(new URL(`../../${path}`, import.meta.url), 'utf8'));
const readText = (path) => readFileSync(new URL(`../../${path}`, import.meta.url), 'utf8');
const bootstrap = read('.github/actions/bootstrap/action.yml');
const ci = read('.github/workflows/ci.yml');
const release = read('.github/workflows/release.yml');
const full = read('.github/workflows/full-checks.yml');
const tauri = read('.github/workflows/tauri-build.yml');
const pages = read('.github/workflows/storybook-pages.yml');
const actionPath = './.github/actions/bootstrap';
const fullPath = './.github/workflows/full-checks.yml';
const configured = (job) => job.steps.find((step) => step.uses === actionPath);

test('Rust cache tracks the single workspace and excludes obsolete source-containing caches', () => {
  const cache = bootstrap.runs.steps.find((step) => step.uses === 'Swatinem/rust-cache@v2');
  assert.equal(cache.with.workspaces.trim(), '. -> target');
  assert.equal(cache.with['prefix-key'], 'v1-rust');
});

test('CI installs the exact Rust toolchain pinned for local development', () => {
  const setup = bootstrap.runs.steps.find((step) =>
    step.uses?.startsWith('dtolnay/rust-toolchain@'),
  );
  const channel = readText('rust-toolchain.toml').match(/^channel\s*=\s*['"]([^'"]+)['"]/m)?.[1];
  assert.ok(channel, 'rust-toolchain.toml must declare an exact channel');
  assert.equal(setup.uses, 'dtolnay/rust-toolchain@stable');
  assert.equal(setup.with.toolchain, channel);
  assert.match(channel, /^\d+\.\d+\.\d+$/);
});

function enabledSteps(options, os) {
  const enabled = (name) => options[name] === 'true';
  const conditions = {
    "inputs.node == 'true'": enabled('node'),
    "inputs.rust == 'true'": enabled('rust'),
    "inputs.security == 'true'": enabled('security'),
    "inputs.rust == 'true' || inputs.security == 'true'": enabled('rust') || enabled('security'),
    "inputs.playwright == 'true'": enabled('playwright'),
    "inputs.rust == 'true' && runner.os == 'Linux'": enabled('rust') && os === 'Linux',
  };
  return bootstrap.runs.steps.filter((step) => {
    if (!step.if) return true;
    assert.ok(Object.hasOwn(conditions, step.if), `Unknown bootstrap condition: ${step.if}`);
    return conditions[step.if];
  });
}

function assertFullEnvironment(action) {
  for (const option of ['node', 'rust', 'playwright', 'security'])
    assert.equal(action.with[option], 'true');
}

test('full-check configuration requires both native and browser dependencies', () => {
  const action = configured(full.jobs.check);
  for (const option of ['node', 'rust', 'playwright', 'security']) {
    const broken = structuredClone(action);
    delete broken.with[option];
    assert.throws(() => assertFullEnvironment(broken));
  }
  assertFullEnvironment(action);
  const steps = enabledSteps(action.with, 'Linux');
  assert.ok(steps.some((step) => step.run?.includes('libwebkit2gtk-4.1-dev')));
  assert.ok(steps.some((step) => step.run === 'task frontend:setup:playwright:ci'));
});

test('release and PR parity execute the identical reusable gate without publishing', () => {
  assert.equal(release.jobs.check.uses, fullPath);
  assert.equal(ci.jobs['release-checks'].uses, fullPath);
  assert.deepEqual(Object.keys(full.on), ['workflow_call']);
  assert.deepEqual(full.permissions, { contents: 'read' });
  assert.equal(full.jobs.check['runs-on'], 'ubuntu-latest');
  assert.equal(full.env.RUSTFLAGS, '-D warnings');
  assert.ok(full.jobs.check.steps.some((step) => step.run === 'task check'));
  assert.ok(full.jobs.check.steps.every((step) => !step['continue-on-error']));
  assert.equal(
    full.jobs.check.steps.some((step) => step.uses?.includes('tauri-action')),
    false,
  );
  assert.equal(release.jobs.build.needs, 'check');
  assert.equal(release.permissions.contents, 'read');
  assert.equal(release.jobs.build.permissions, undefined);
  assert.equal(release.jobs.publish.needs, 'build');
  assert.equal(release.jobs.publish.permissions.contents, 'write');
  assert.deepEqual(release.on.push.tags, ['app-v*']);
  assert.ok(ci.jobs['ci-summary'].needs.includes('release-checks'));
});

test('all toolchain consumers use the same bootstrap after checkout', () => {
  const jobs = [
    ci.jobs.frontend,
    ci.jobs.rust,
    ci.jobs['crash-recovery'],
    ci.jobs.docs,
    ci.jobs['quality-global'],
    ci.jobs.security,
    full.jobs.check,
    release.jobs.build,
    release.jobs.publish,
    tauri.jobs.build,
    pages.jobs.build,
  ];
  for (const job of jobs) {
    const steps = job.steps;
    const setup = steps.findIndex((step) => step.uses === actionPath);
    const checkout = steps.findIndex((step) => step.uses?.startsWith('actions/checkout@'));
    assert.ok(checkout >= 0 && setup > checkout);
    assert.equal(steps.filter((step) => step.uses === actionPath).length, 1);
    assert.ok(
      steps.every(
        (step) =>
          !/pnpm\/setup|setup-node|rust-toolchain|rust-cache|setup-task/.test(step.uses ?? ''),
      ),
    );
    assert.ok(
      steps.every((step) => !/apt-get|playwright install|task install/.test(step.run ?? '')),
    );
    assert.ok(steps.slice(0, setup).every((step) => !step.run));
  }
});

test('dependency installation is ordered and frozen before browser installation', () => {
  assert.equal(bootstrap.runs.using, 'composite');
  for (const step of bootstrap.runs.steps) {
    if (step.run) assert.equal(step.shell, 'bash');
  }
  const steps = enabledSteps({ node: 'true', rust: 'true', playwright: 'true' }, 'Linux');
  const index = (predicate) => {
    const result = steps.findIndex(predicate);
    assert.ok(result >= 0);
    return result;
  };
  const task = index((step) => step.uses?.startsWith('go-task/setup-task@'));
  const dependencies = index((step) => step.run === 'task install:frontend');
  const browser = index((step) => step.run === 'task frontend:setup:playwright:ci');
  assert.ok(task < dependencies && dependencies < browser);
  assert.ok(index((step) => step.uses?.startsWith('pnpm/setup@')) < dependencies);
  assert.ok(
    index((step) => step.uses?.startsWith('dtolnay/rust-toolchain@')) <
      index((step) => step.run === 'task install:rust'),
  );
  assert.equal(
    read('taskfiles/frontend.yml').tasks.install.cmds[0],
    'pnpm install --frozen-lockfile',
  );
  assert.equal(read('taskfiles/rust.yml').tasks.fetch.cmds[0], 'cargo fetch --locked');
  assert.equal(
    read('taskfiles/frontend.yml').tasks['setup:playwright:ci'].cmds[0],
    'pnpm --filter desktop exec playwright install --with-deps chromium',
  );
});

test('native crash recovery is required on Windows and macOS as well as the Linux Rust gate', () => {
  const job = ci.jobs['crash-recovery'];
  assert.deepEqual(job.strategy.matrix.os, ['windows-latest', 'macos-latest']);
  assert.deepEqual(configured(job).with, { rust: 'true' });
  assert.ok(job.steps.some((step) => step.run === 'task rs:test:storage'));
  assert.ok(job.steps.some((step) => step.run === 'task rs:test:youtube'));
  assert.ok(ci.jobs['ci-summary'].needs.includes('crash-recovery'));
  assert.ok(job.steps.every((step) => !step['continue-on-error']));
});

test('Node version and Tauri dependency list have one shared definition', () => {
  const node = bootstrap.runs.steps.find((step) => step.uses?.startsWith('pnpm/setup@'));
  assert.equal(node.with.runtime, 'node@24');
  const cache = bootstrap.runs.steps.find((step) => step.uses?.startsWith('actions/setup-node@'));
  assert.equal(cache.with['node-version'], undefined);
  const linux = enabledSteps({ rust: 'true' }, 'Linux').find((step) =>
    step.run?.includes('apt-get'),
  );
  for (const dependency of [
    'libwebkit2gtk-4.1-dev',
    'libappindicator3-dev',
    'librsvg2-dev',
    'patchelf',
    'xdg-utils',
  ]) {
    assert.ok(linux.run.includes(dependency));
  }
});

test('lightweight jobs skip unneeded browsers, native libraries and Rust fetching', () => {
  assert.deepEqual(configured(ci.jobs.frontend).with, { node: 'true', playwright: 'true' });
  assert.deepEqual(configured(ci.jobs.rust).with, { rust: 'true' });
  for (const job of [ci.jobs.docs, ci.jobs['quality-global'], pages.jobs.build]) {
    assert.deepEqual(configured(job).with, { node: 'true' });
    const steps = enabledSteps(configured(job).with, 'Linux');
    assert.equal(
      steps.some((step) => /playwright|apt-get|install:rust|install:all/.test(step.run ?? '')),
      false,
    );
  }
  for (const job of [release.jobs.build, tauri.jobs.build]) {
    assert.deepEqual(configured(job).with, { node: 'true', rust: 'true' });
    for (const os of ['Windows', 'macOS']) {
      const steps = enabledSteps(configured(job).with, os);
      assert.equal(
        steps.some((step) => /apt-get|playwright/.test(step.run ?? '')),
        false,
      );
    }
  }
});

test('bootstrap and tooling changes trigger the release gate and cannot bypass CI Summary', () => {
  const filters = load(ci.jobs.changes.steps.find((step) => step.id === 'filter').with.filters);
  assert.ok(filters.ci.includes('.github/actions/**'));
  assert.ok(filters.ci.includes('.github/workflows/**'));
  assert.ok(filters.quality.includes('taskfiles/**'));
  assert.ok(filters.rust.includes('rust-toolchain.toml'));
  assert.ok(filters.release.includes('src-tauri/tauri.*.conf.json'));
  assert.ok(filters.release.includes('tools/release/**'));
  assert.ok(filters.release.includes('taskfiles/release.yml'));
  for (const group of ['ci', 'quality', 'global', 'release']) {
    assert.ok(ci.jobs['release-checks'].if.includes(`needs.changes.outputs.${group} == 'true'`));
  }
  assert.equal(ci.jobs['ci-summary'].if, 'always()');
  assert.ok(ci.jobs['ci-summary'].steps[0].run.includes("contains(needs.*.result, 'failure')"));
  assert.ok(ci.jobs['ci-summary'].steps[0].run.includes("contains(needs.*.result, 'cancelled')"));
});

test('vendored GLib changes require source integrity and optimized Linux regression gates', () => {
  const filters = load(ci.jobs.changes.steps.find((step) => step.id === 'filter').with.filters);
  for (const group of ['rust', 'quality']) {
    assert.ok(filters[group].includes('vendor/glib-0.18.5/**'));
    assert.ok(filters[group].includes('tools/glib-backport/**'));
  }
  assert.ok(ci.jobs.rust.steps.some((step) => step.run === 'task rs:glib:reproduce'));
});

test('SQLite dependency guards run in both the lightweight and resolved Rust gates', () => {
  assert.ok(
    read('taskfiles/quality.yml').tasks.global.cmds.some(
      (command) => command.task === ':sec:sqlite:verify',
    ),
  );
  assert.ok(
    read('taskfiles/rust.yml').tasks.all.cmds.some(
      (command) => command.task === 'dependencies:verify',
    ),
  );
  assert.equal(
    read('taskfiles/rust.yml').tasks['dependencies:verify'].cmds[0],
    'node tools/scripts/check-sqlite-dependencies.mjs --resolved',
  );
});
