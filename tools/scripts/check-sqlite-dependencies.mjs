import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const sqlxPackages = ['sqlx', 'sqlx-core', 'sqlx-macros', 'sqlx-macros-core', 'sqlx-sqlite'];
const forbidden = new Set(['rsa', 'sqlx-mysql', 'sqlx-postgres']);
const facadeFeatures = [
  '_rt-tokio',
  '_sqlite',
  'derive',
  'runtime-tokio',
  'sqlite-bundled',
  'sqlx-macros',
  'sqlx-sqlite',
];

export function checkPackages(packages) {
  for (const entry of packages) {
    assert.ok(!forbidden.has(entry.name), `Unneeded database dependency returned: ${entry.name}`);
  }
  const sqlx = packages.filter((entry) => entry.name === 'sqlx' || entry.name.startsWith('sqlx-'));
  assert.deepEqual(sqlx.map((entry) => entry.name).sort(), sqlxPackages);
  for (const entry of sqlx) {
    assert.match(entry.version, /^0\.9\.\d+$/, `Unexpected SQLx version for ${entry.name}`);
    assert.equal(entry.version, sqlx[0].version, 'SQLx packages must use one compatible release');
  }
}

export function checkLockfile(source) {
  const blocks = source.split(/^\[\[package\]\]\r?$/m).slice(1);
  assert.ok(blocks.length > 0, 'Cargo.lock contains no package records');
  const packages = blocks.map((block) => {
    const name = /^name = "([^"]+)"\r?$/m.exec(block)?.[1];
    const version = /^version = "([^"]+)"\r?$/m.exec(block)?.[1];
    assert.ok(name && version, 'Malformed Cargo.lock package record');
    return { name, version };
  });
  checkPackages(packages);
}

export function checkResolved(metadata) {
  checkPackages(metadata.packages);
  const facade = metadata.packages.find((entry) => entry.name === 'sqlx');
  const driver = metadata.packages.find((entry) => entry.name === 'sqlx-sqlite');
  const features = (entry) => {
    const node = metadata.resolve.nodes.find((node) => node.id === entry.id);
    assert.ok(node, `Dependency is not resolved: ${entry.name}`);
    return node.features;
  };
  assert.deepEqual([...features(facade)].sort(), facadeFeatures);
  assert.ok(features(driver).includes('chrono'), 'SQLite must retain its timestamp encoding');
  assert.ok(features(driver).includes('bundled'), 'SQLite must not depend on a system library');
  for (const feature of ['load-extension', 'deserialize', 'unbundled', 'preupdate-hook']) {
    assert.ok(!features(driver).includes(feature), `Unused SQLite feature returned: ${feature}`);
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const root = fileURLToPath(new URL('../../', import.meta.url));
  checkLockfile(readFileSync(path.join(root, 'Cargo.lock'), 'utf8'));
  if (process.argv.includes('--resolved')) {
    const metadata = execFileSync('cargo', ['metadata', '--locked', '--format-version', '1'], {
      cwd: root,
      encoding: 'utf8',
      maxBuffer: 32 * 1024 * 1024,
      stdio: ['ignore', 'pipe', 'inherit'],
    });
    checkResolved(JSON.parse(metadata));
  }
  console.log('SQLite-only SQLx dependencies verified; no RSA, MySQL or PostgreSQL packages.');
}
