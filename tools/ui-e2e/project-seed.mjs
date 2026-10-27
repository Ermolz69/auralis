import assert from 'node:assert/strict';
import fsp from 'node:fs/promises';
import fs from 'node:fs';
import path from 'node:path';
import { root, storage, owned, json, hash, idPattern } from './core.mjs';
import { requireQuiescent } from './processes.mjs';
import { runInstaller } from './installer.mjs';
import { validateSeedPath } from './seed.mjs';
import { relocateTts } from './relocate.mjs';

const [id, inputRoot, inputId, manifestSha, buildPython, contractRoot] = process.argv.slice(2);
assert.ok(idPattern.test(id ?? '') && idPattern.test(inputId ?? ''));
assert.match(manifestSha ?? '', /^[a-f0-9]{64}$/);
const source = owned(
  path.join(inputRoot, '.cache/ui-e2e/seeds'),
  path.join(inputRoot, '.cache/ui-e2e/seeds', inputId),
);
assert.equal(await hash(`${source}.json`), manifestSha, 'Pinned input manifest changed');
const input = await json(`${source}.json`, 32 * 1024 * 1024);
const config = await json(path.join(source, 'tts-runtime.json'));
const python = owned(source, config.python_executable);
const prefix = `${path.relative(source, path.dirname(python)).replaceAll('\\', '/')}/`;
const packagePrefix = `${path.relative(source, config.package_root).replaceAll('\\', '/')}/`;
const inventory = await json(path.join(owned(source, config.package_root), 'integrity.json'));
const dependencies = new Set(
  inventory.dependencies.map((item) =>
    path.relative(source, owned(source, item.path)).replaceAll('\\', '/'),
  ),
);
dependencies.add(`${prefix.replace(/base\/$/, 'venv/')}pyvenv.cfg`);
dependencies.add('tts-runtime.json');
const files = input.files.filter((file) => {
  const relative = file.path.replaceAll('\\', '/');
  return (
    relative.startsWith(prefix) || relative.startsWith(packagePrefix) || dependencies.has(relative)
  );
});
assert.ok(files.length > 0 && files.length <= 10000);
assert.ok(files.reduce((bytes, file) => bytes + file.size_bytes, 0) <= 256 * 1024 ** 2);
const directory = owned(path.join(storage, 'seeds'), path.join(storage, 'seeds', id));
await fsp.mkdir(path.dirname(directory), { recursive: true });
await fsp.mkdir(directory);
const deadline = Date.now() + 300000;
const remaining = () => {
  assert.ok(Date.now() < deadline, 'Media seed deadline exhausted');
  return deadline - Date.now();
};
for (const file of files) {
  validateSeedPath(file.path);
  const origin = owned(source, path.join(source, file.path));
  assert.equal(await fsp.realpath(origin), origin);
  assert.ok((await fsp.lstat(origin)).isFile());
  assert.equal((await fsp.stat(origin)).size, file.size_bytes);
  assert.equal(await hash(origin, remaining), file.sha256);
  const target = owned(directory, path.join(directory, file.path));
  await fsp.mkdir(path.dirname(target), { recursive: true });
  await fsp.copyFile(origin, target, fs.constants.COPYFILE_EXCL);
}
const remap = (value) => {
  if (typeof value === 'string' && path.isAbsolute(value))
    return owned(directory, path.join(directory, path.relative(source, owned(source, value))));
  if (Array.isArray(value)) return value.map(remap);
  if (value && typeof value === 'object')
    return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, remap(item)]));
  return value;
};
await fsp.writeFile(
  path.join(directory, 'tts-runtime.json'),
  JSON.stringify(remap(config), null, 2),
);
await relocateTts(directory, source, remaining);
await requireQuiescent();
const log = await fsp.open(path.join(storage, 'seeds', `${id}.setup.log`), 'wx');
try {
  await runInstaller(
    buildPython,
    [
      '-I',
      '-B',
      path.join(root, 'tools/tts/setup_project.py'),
      '--data-root',
      directory,
      '--contract-root',
      contractRoot,
      '--runtime-python',
      path.join(directory, path.relative(source, python)),
    ],
    {
      windowsHide: true,
      stdio: ['ignore', log.fd, log.fd],
      env: {
        ...process.env,
        PIP_NO_INDEX: '1',
        PYTHONDONTWRITEBYTECODE: '1',
      },
    },
    remaining,
  );
} finally {
  await log.close();
}
const manifest = { directory: path.relative(root, directory).replaceAll('\\', '/'), files: [] };
async function scan(folder) {
  for (const entry of await fsp.readdir(folder, { withFileTypes: true })) {
    const file = owned(directory, path.join(folder, entry.name));
    if (entry.isDirectory()) await scan(file);
    else {
      const relative = path.relative(directory, file).replaceAll('\\', '/');
      validateSeedPath(relative);
      manifest.files.push({
        path: relative,
        size_bytes: (await fsp.stat(file)).size,
        sha256: await hash(file, remaining),
      });
    }
  }
}
await scan(directory);
await fsp.writeFile(`${directory}.json`, JSON.stringify(manifest, null, 2), { flag: 'wx' });
process.stdout.write(
  `Prepared metadata-only media seed ${id}: ${manifest.files.length} pinned files; engine dependencies, voices and model weights are absent\n`,
);
