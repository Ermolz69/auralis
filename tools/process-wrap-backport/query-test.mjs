import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { cp, mkdtemp, readFile, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));
const vendor = path.join(root, 'vendor/process-wrap-10.0.0');
const provenance = JSON.parse(await readFile(new URL('./provenance.json', import.meta.url)));
for (const [name, expected] of Object.entries(provenance.patchedFiles)) {
  assert.equal(
    createHash('sha256')
      .update(await readFile(path.join(vendor, name)))
      .digest('hex'),
    expected,
  );
}
const directory = await mkdtemp(path.join(os.tmpdir(), 'auralis-job-query-test-'));
const source = path.join(directory, 'process-wrap-10.0.0');
await cp(vendor, source, { recursive: true, force: false, errorOnExist: true });
// Unit-test the exact maintained Rust sources without resolving unrelated upstream
// integration-test libraries. The shipped package and its Cargo.lock stay untouched.
const manifest = (await readFile(path.join(source, 'Cargo.toml'), 'utf8')).replace(
  /\n\[dev-dependencies(?:\.[^\]]+)?\][\s\S]*?(?=\n\[|$)/g,
  '\n',
);
await writeFile(path.join(source, 'Cargo.toml'), manifest);
await writeFile(path.join(source, 'Cargo.lock'), await readFile(path.join(root, 'Cargo.lock')));
const result = spawnSync(
  'cargo',
  [
    'test',
    '--offline',
    '--manifest-path',
    path.join(source, 'Cargo.toml'),
    '--lib',
    '--no-default-features',
    '--features',
    'tokio1,job-object,kill-on-drop,creation-flags',
    '--',
    'job_query',
  ],
  { stdio: 'inherit', windowsHide: true, timeout: 300000 },
);
if (result.error) throw result.error;
assert.equal(result.signal, null);
process.exitCode = result.status;
