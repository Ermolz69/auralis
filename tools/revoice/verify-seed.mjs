import assert from 'node:assert/strict';
import fsp from 'node:fs/promises';
import path from 'node:path';
import { storage, root, owned, hash, json, idPattern } from '../ui-e2e/core.mjs';
import { validateSeedPath } from '../ui-e2e/seed.mjs';

const [id, seedId, pin] = process.argv.slice(2);
assert.ok(idPattern.test(id ?? '') && idPattern.test(seedId ?? ''));
assert.match(pin ?? '', /^[a-f0-9]{64}$/);
const directory = owned(
  path.join(storage, 'component-checks'),
  path.join(storage, 'component-checks', id),
);
await fsp.mkdir(path.dirname(directory), { recursive: true });
await fsp.mkdir(directory);
const report = {
  version: 1,
  status: 'not_run',
  scope: 'Full seed byte validation only',
  models_loaded: false,
};
const started = Date.now();
try {
  const file = path.join(storage, 'seeds', `${seedId}.json`);
  assert.equal(await hash(file), pin, 'Seed manifest changed');
  const manifest = await json(file, 32 * 1024 * 1024);
  const source = owned(path.join(storage, 'seeds'), path.resolve(root, manifest.directory));
  assert.equal(await fsp.realpath(source), source);
  assert.ok(manifest.files.length > 0 && manifest.files.length <= 65536);
  const seen = new Set();
  let bytes = 0,
    next = 0,
    verified = 0;
  for (const entry of manifest.files) {
    validateSeedPath(entry.path);
    assert.ok(!seen.has(entry.path.toLowerCase()), 'Duplicate seed file');
    seen.add(entry.path.toLowerCase());
    assert.ok(Number.isSafeInteger(entry.size_bytes) && entry.size_bytes >= 0);
    bytes += entry.size_bytes;
  }
  assert.ok(bytes <= 32 * 1024 ** 3);
  await Promise.all(
    Array.from({ length: 8 }, async () => {
      while (next < manifest.files.length) {
        assert.ok(Date.now() - started < 1800000, 'Seed verification deadline exhausted');
        const entry = manifest.files[next++];
        const target = owned(source, path.join(source, entry.path));
        assert.equal(await fsp.realpath(target), target);
        assert.equal((await fsp.stat(target)).size, entry.size_bytes);
        assert.equal(await hash(target), entry.sha256, `Changed seed file: ${entry.path}`);
        if (++verified % 4000 === 0)
          process.stdout.write(`Verified ${verified}/${manifest.files.length} files\n`);
      }
    }),
  );
  Object.assign(report, { status: 'passed', manifest_sha256: pin, files: verified, bytes });
} catch (error) {
  report.status = 'failed';
  report.error = String(error.message);
  process.exitCode = 1;
} finally {
  report.elapsed_ms = Date.now() - started;
  await fsp.writeFile(path.join(directory, 'report.json'), JSON.stringify(report, null, 2), {
    flag: 'wx',
  });
  process.stdout.write(`Seed validation: ${id} ${report.status}\n`);
}
