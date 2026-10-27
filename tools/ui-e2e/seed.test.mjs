import test from 'node:test';
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fsp from 'node:fs/promises';
import path from 'node:path';
import { root, storage, hash, owned, json, Blocked } from './core.mjs';
import { seedProfile } from './seed.mjs';

test('component relocation preserves voice bytes and pins the new profile inventory', async () => {
  const id = `relocate-test-${crypto.randomUUID()}`;
  const directory = owned(storage, path.join(storage, 'seeds', id));
  const output = owned(storage, path.join(storage, 'runs', id));
  const runtime = path.join(directory, 'tts/runtime/fixture');
  const packages = path.join(directory, 'tts/packages/fixture');
  const voice = 'tts/voice-stores/fixture/voice/voice.json';
  const put = async (relative, contents) => {
    const file = owned(directory, path.join(directory, relative));
    await fsp.mkdir(path.dirname(file), { recursive: true });
    await fsp.writeFile(file, contents);
  };
  await put('tts/runtime/fixture/base/python.exe', 'fixture-only');
  await put('tts/runtime/fixture/venv/pyvenv.cfg', `home = ${path.join(runtime, 'base')}\n`);
  await put('tts/runtime/fixture/venv/METADATA', 'fixture dependency');
  await put(voice, '{"qualification":"unverified","audio_sha256":"unchanged"}');
  await put('tts/packages/fixture/src/tts/worker.py', 'fixture-only');
  const dependency = path.join(runtime, 'venv/METADATA');
  await put(
    'tts/packages/fixture/integrity.json',
    JSON.stringify({
      version: 1,
      package_files: [],
      model_files: [],
      dependencies: [{ path: dependency, sha256: await hash(dependency) }],
    }),
  );
  await put(
    'tts-runtime.json',
    JSON.stringify({
      version: 1,
      python_executable: path.join(runtime, 'base/python.exe'),
      package_root: packages,
      voice_store: path.join(directory, 'tts/voice-stores/fixture'),
      profile_root: path.join(packages, 'share/puza-tts/profiles'),
      model_cache: path.join(directory, 'tts/model-cache'),
      integrity_sha256: await hash(path.join(packages, 'integrity.json')),
    }),
  );
  const originalConfig = await fsp.readFile(path.join(directory, 'tts-runtime.json'));
  await put('project-contract/packages/fixture/metadata.json', '{"version":"0.3.0"}');
  await put(
    'project-runtime.json',
    JSON.stringify({
      version: 1,
      python_executable: path.join(runtime, 'base/python.exe'),
      package_root: path.join(directory, 'project-contract/packages/fixture'),
      package_version: '0.3.0',
      files: [
        {
          path: 'metadata.json',
          sha256: await hash(
            path.join(directory, 'project-contract/packages/fixture/metadata.json'),
          ),
        },
      ],
    }),
  );
  const files = [];
  const scan = async (folder) => {
    for (const item of await fsp.readdir(folder, { withFileTypes: true })) {
      const file = path.join(folder, item.name);
      if (item.isDirectory()) await scan(file);
      else
        files.push({
          path: path.relative(directory, file),
          size_bytes: (await fsp.stat(file)).size,
          sha256: await hash(file),
        });
    }
  };
  await scan(directory);
  const result = await seedProfile(
    {
      runtime_seed: { directory: path.relative(root, directory), files },
      tts_execution_budget_seconds: 2700,
    },
    { directory: output },
  );
  assert.equal(result.qualification_modified, false);
  const profile = path.join(output, 'profile');
  const config = await json(path.join(profile, 'tts-runtime.json'));
  assert.equal(config.execution_budget_seconds, 2700);
  const mediaConfig = await json(path.join(profile, 'project-runtime.json'));
  assert.equal(mediaConfig.package_root, path.join(profile, 'project-contract/packages/fixture'));
  assert.equal(
    mediaConfig.python_executable,
    path.join(profile, 'tts/runtime/fixture/base/python.exe'),
  );
  assert.equal(mediaConfig.package_version, '0.3.0');
  assert.equal(
    await hash(path.join(mediaConfig.package_root, 'metadata.json')),
    mediaConfig.files[0].sha256,
  );
  assert.equal(result.tts_runtime_sha256, await hash(path.join(profile, 'tts-runtime.json')));
  assert.ok(config.package_root.startsWith(profile + path.sep));
  const inventoryFile = path.join(config.package_root, 'integrity.json');
  assert.equal(await hash(inventoryFile), config.integrity_sha256);
  const inventory = await json(inventoryFile);
  assert.equal(
    inventory.dependencies[0].path,
    path.join(profile, 'tts/runtime/fixture/venv/METADATA'),
  );
  assert.equal(await hash(path.join(directory, voice)), await hash(path.join(profile, voice)));
  assert.deepEqual(await fsp.readFile(path.join(directory, 'tts-runtime.json')), originalConfig);
  assert.match(
    await fsp.readFile(path.join(profile, 'tts/runtime/fixture/venv/pyvenv.cfg'), 'utf8'),
    /runs.*relocate-test/,
  );
  await put(voice, 'corrupted synthetic voice fixture');
  await assert.rejects(
    seedProfile(
      { runtime_seed: { directory: path.relative(root, directory), files } },
      { directory: owned(storage, path.join(storage, 'runs', `${id}-corrupt`)) },
    ),
    (error) => !(error instanceof Blocked) && /Seed integrity changed/.test(error.message),
  );
});
