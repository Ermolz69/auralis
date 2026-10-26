import fsp from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import { root, hash, json, owned, idPattern, UnconfirmedCleanup } from './core.mjs';
import { runInstaller } from './installer.mjs';

export async function verifyPcmReadOnly(
  profile,
  run,
  input,
  spec,
  remaining,
  signal,
  onOwned,
  admit,
) {
  assert.match(input.run_id, idPattern);
  const config = await json(path.join(profile, 'tts-runtime.json'));
  const inventoryFile = owned(profile, path.join(config.package_root, 'integrity.json'));
  assert.equal(await hash(inventoryFile, remaining), config.integrity_sha256);
  const inventory = await json(inventoryFile);
  const helper = owned(profile, path.join(config.package_root, 'tts/audio/joiner.py'));
  const entry = inventory.package_files.find(
    (item) => item.path.replaceAll('\\', '/') === 'tts/audio/joiner.py',
  );
  assert.ok(entry, 'Pinned streaming helper is unavailable');
  assert.equal(await hash(helper, remaining), entry.sha256);
  const runtime = path.dirname(path.dirname(config.python_executable));
  const site_packages = owned(profile, path.join(runtime, 'venv/Lib/site-packages'));
  const library_files = spec.runtime_seed.files
    .filter((item) =>
      /\/venv\/Lib\/site-packages\/(numpy|scipy|soundfile|_soundfile|_cffi_backend|cffi)/i.test(
        item.path.replaceAll('\\', '/'),
      ),
    )
    .map((item) => ({ path: owned(profile, path.join(profile, item.path)), sha256: item.sha256 }));
  for (const name of ['numpy', 'scipy', 'soundfile'])
    assert.ok(
      library_files.some((item) =>
        path.relative(site_packages, item.path).toLowerCase().startsWith(name),
      ),
      `Pinned ${name} library is unavailable`,
    );
  const file = path.join(run.directory, `pcm-${input.run_id}.json`),
    result = path.join(run.directory, `pcm-${input.run_id}-result.json`);
  await fsp.writeFile(
    file,
    JSON.stringify({
      ...input,
      site_packages,
      helper_path: helper,
      helper_sha256: entry.sha256,
      library_files,
      max_wall_seconds: Math.max(1, Math.min(300, Math.floor(remaining() / 1000))),
    }),
    { flag: 'wx' },
  );
  const release = await admit();
  let log, failure;
  try {
    log = await fsp.open(path.join(run.directory, `pcm-${input.run_id}.log`), 'wx');
    await runInstaller(
      owned(profile, config.python_executable),
      ['-I', '-S', '-B', path.join(root, 'tools/ui-e2e/pcm-verify.py'), file, result],
      {
        windowsHide: true,
        stdio: ['ignore', log.fd, log.fd],
        env: { ...process.env, PYTHONHOME: '', PYTHONPATH: '', PYTHONDONTWRITEBYTECODE: '1' },
      },
      remaining,
      signal,
      onOwned,
    );
    const facts = await json(result);
    assert.equal(facts.models_loaded, false);
    assert.equal(facts.pcm_sha256, facts.expected_pcm_sha256);
    return { ...facts, report: path.basename(result) };
  } catch (error) {
    failure = error;
    if (error instanceof UnconfirmedCleanup) throw error;
    throw new Error(
      `Read-only PCM verification failed; inspect pcm-${input.run_id}.log: ${error.message}`,
    );
  } finally {
    try {
      await log?.close();
    } finally {
      if (!(failure instanceof UnconfirmedCleanup)) await release();
    }
  }
}
