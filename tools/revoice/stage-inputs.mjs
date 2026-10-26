import assert from 'node:assert/strict';
import fsp from 'node:fs/promises';
import path from 'node:path';
import fs from 'node:fs';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';

const root = fileURLToPath(new URL('../../', import.meta.url));
const [sourceArgument, sourceSeedId, seedId, componentsId, manifestPin] = process.argv.slice(2);
for (const id of [sourceSeedId, seedId, componentsId])
  assert.match(id ?? '', /^[A-Za-z0-9][A-Za-z0-9._-]{0,120}$/);
assert.match(manifestPin ?? '', /^[a-f0-9]{64}$/);
const sourceRoot = await fsp.realpath(sourceArgument);
const sourceSeed = path.join(sourceRoot, '.cache/ui-e2e/seeds', sourceSeedId);
const seed = path.join(root, '.cache/ui-e2e/seeds', seedId);
const manifestBytes = await fsp.readFile(`${sourceSeed}.json`);
const sha = (bytes) => createHash('sha256').update(bytes).digest('hex');
assert.equal(sha(manifestBytes), manifestPin, 'Source seed manifest changed');
const manifest = JSON.parse(manifestBytes);
assert.equal(path.resolve(sourceRoot, manifest.directory), sourceSeed);
assert.ok(manifest.files.length > 0 && manifest.files.length <= 65536);
const files = new Set();
let total = 0;
for (const file of manifest.files) {
  assert.ok(typeof file.path === 'string' && !path.isAbsolute(file.path));
  assert.ok(
    !file.path
      .split(/[\\/]/)
      .some((part) => !part || part === '..' || part === '.' || part.includes(':')),
  );
  assert.ok(!files.has(file.path.toLowerCase()), 'Duplicate seed file');
  files.add(file.path.toLowerCase());
  assert.ok(Number.isSafeInteger(file.size_bytes) && file.size_bytes >= 0);
  assert.match(file.sha256, /^[a-f0-9]{64}$/);
  total += file.size_bytes;
}
assert.ok(total <= 32 * 1024 ** 3, 'Seed exceeds 32 GiB');
await fsp.mkdir(path.dirname(seed), { recursive: true });
await fsp.mkdir(seed);
const started = Date.now();
const deadline = started + 1800000;
let next = 0;
async function copy(source, target, expectedBytes) {
  assert.ok(Date.now() < deadline, 'Staging deadline exhausted');
  const stat = await fsp.lstat(source);
  assert.ok(stat.isFile() && !stat.isSymbolicLink());
  assert.equal(await fsp.realpath(source), source, 'Redirected source');
  if (expectedBytes !== undefined) assert.equal(stat.size, expectedBytes);
  await fsp.mkdir(path.dirname(target), { recursive: true });
  await fsp.copyFile(source, target, fs.constants.COPYFILE_EXCL);
  assert.equal((await fsp.stat(target)).size, stat.size);
}
await Promise.all(
  Array.from({ length: 16 }, async () => {
    while (next < manifest.files.length) {
      const file = manifest.files[next++];
      await copy(path.join(sourceSeed, file.path), path.join(seed, file.path), file.size_bytes);
      if (next % 2000 === 0)
        process.stdout.write(`Staged ${next}/${manifest.files.length} files\n`);
    }
  }),
);
const remap = (value) => {
  if (Array.isArray(value)) return value.map(remap);
  if (value && typeof value === 'object')
    return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, remap(item)]));
  if (typeof value === 'string' && path.isAbsolute(value)) {
    const relative = path.relative(sourceSeed, value);
    assert.ok(
      relative && !relative.startsWith('..') && !path.isAbsolute(relative),
      'Configuration escapes source seed',
    );
    return path.join(seed, relative);
  }
  return value;
};
const configFile = path.join(seed, 'tts-runtime.json');
const config = remap(JSON.parse(await fsp.readFile(configFile, 'utf8')));
const inventoryFile = path.join(config.package_root, 'integrity.json');
const inventory = remap(JSON.parse(await fsp.readFile(inventoryFile, 'utf8')));
await fsp.writeFile(inventoryFile, JSON.stringify(inventory, null, 2));
config.integrity_sha256 = sha(await fsp.readFile(inventoryFile));
await fsp.writeFile(configFile, JSON.stringify(config, null, 2));
for (const target of [configFile, inventoryFile]) {
  const relative = path.relative(seed, target).replaceAll('\\', '/');
  const entry = manifest.files.find((file) => file.path.replaceAll('\\', '/') === relative);
  assert.ok(entry, 'Changed configuration was not in the manifest');
  entry.sha256 = sha(await fsp.readFile(target));
  entry.size_bytes = (await fsp.stat(target)).size;
}
manifest.directory = path.relative(root, seed).replaceAll('\\', '/');
await fsp.writeFile(`${seed}.json`, JSON.stringify(manifest, null, 2), { flag: 'wx' });
const wheels = path.join('.cache/ui-e2e/wheels', componentsId);
const receipt = JSON.parse(
  await fsp.readFile(path.join(sourceRoot, wheels, 'components.json'), 'utf8'),
);
await fsp.mkdir(path.join(root, wheels), { recursive: true });
for (const file of receipt.files) {
  assert.equal(path.dirname(file.path).replaceAll('\\', '/'), wheels.replaceAll('\\', '/'));
  const source = path.join(sourceRoot, file.path);
  assert.equal(sha(await fsp.readFile(source)), file.sha256);
  await copy(source, path.join(root, file.path), file.size_bytes);
}
await copy(
  path.join(sourceRoot, wheels, 'components.json'),
  path.join(root, wheels, 'components.json'),
);
for (const name of await fsp.readdir(path.join(sourceRoot, 'src-tauri/binaries')))
  await copy(
    path.join(sourceRoot, 'src-tauri/binaries', name),
    path.join(root, 'src-tauri/binaries', name),
  );
await fsp.writeFile(
  path.join(seed, '..', `${seedId}.staging.json`),
  JSON.stringify(
    {
      version: 1,
      status: 'passed',
      scope: 'Owned file staging only; full seed hash validation remains required before use',
      files: manifest.files.length,
      bytes: total,
      elapsed_ms: Date.now() - started,
      source_manifest_sha256: manifestPin,
      manifest_sha256: sha(await fsp.readFile(`${seed}.json`)),
      full_hash_validation: 'not_run',
      models_loaded: false,
    },
    null,
    2,
  ),
  { flag: 'wx' },
);
process.stdout.write(
  'Staged owned inputs without loading models; full seed validation remains required.\n',
);
