import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { root, upstream, vendor, verifyTrees, patchManifest } from './package.mjs';

assert.equal(process.platform, 'linux', 'The regression must execute against Linux GLib');
const original = await upstream();
await verifyTrees(original, vendor);
const metadata = JSON.parse(
  execFileSync('cargo', ['metadata', '--locked', '--format-version', '1'], {
    cwd: root,
    encoding: 'utf8',
    maxBuffer: 32 * 1024 * 1024,
  }),
);
const resolved = metadata.packages.filter((entry) => entry.name === 'glib');
assert.equal(resolved.length, 1);
assert.equal(resolved[0].source, null);
assert.equal(resolved[0].manifest_path, path.join(vendor, 'Cargo.toml'));
const common = ['test', '--locked', '--release', '-p', 'glib-backport-regression'];
const positive = spawnSync('cargo', common, { cwd: root, stdio: 'inherit' });
assert.equal(positive.status, 0, 'Patched optimized regression must pass first');

const config = `patch.crates-io.glib.path=${JSON.stringify(original)}`;
const originalManifest = path.join(original, 'Cargo.toml');
await writeFile(originalManifest, patchManifest(await readFile(originalManifest, 'utf8')));
const negative = spawnSync(
  'cargo',
  [...common, '--config', config, '--', '--exact', 'tests::next_reads_the_ffi_out_pointer'],
  {
    cwd: root,
    encoding: 'utf8',
    maxBuffer: 16 * 1024 * 1024,
    env: {
      ...process.env,
      CARGO_TARGET_DIR: path.join(
        process.env.CARGO_TARGET_DIR ?? path.join(root, 'target'),
        'glib-negative',
      ),
    },
  },
);
const output = `${negative.stdout ?? ''}\n${negative.stderr ?? ''}`;
console.log(output.slice(-8000));
assert.ok(!negative.error, negative.error?.message);
assert.match(output, /Running unittests/, 'A compile/setup failure is not evidence of the defect');
assert.notEqual(
  negative.status,
  0,
  'The unpatched control unexpectedly passed; investigate before accepting the backport',
);
assert.match(
  output,
  /signal: (6, SIGABRT|11, SIGSEGV)/,
  'Expected the upstream invalid-pointer failure',
);
console.log(
  'Unpatched GLib reproduces the optimized pointer crash; patched GLib passes the same test.',
);
