import fsp from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import { root, uniqueRun, owned, json, hash, idPattern, UnconfirmedCleanup } from './core.mjs';
import { lockSession } from './session.mjs';
import { acquireDownloadAdmission } from './admission.mjs';
import { requireQuiescent } from './processes.mjs';
import { runInstaller } from './installer.mjs';
import { Report } from './report.mjs';

const [seedId, componentId, inputRoot = root] = process.argv.slice(2);
assert.ok([seedId, componentId].every((id) => idPattern.test(id ?? '')));
const base = path.resolve(inputRoot),
  storage = path.join(base, '.cache/ui-e2e');
const run = await uniqueRun('pcm-model-free');
const report = new Report(run, 'synthetic-read-only-pcm', ['pcm-regression', 'cleanup']);
const deadline = Date.now() + 300000;
const remaining = () => {
  const ms = deadline - Date.now();
  if (ms <= 0) throw new Error('PCM regression deadline exhausted');
  return ms;
};
let release,
  admission,
  log,
  confirmed = true;
await report.step('pcm-regression', async () => {
  release = await lockSession();
  await requireQuiescent();
  const seed = await json(path.join(storage, 'seeds', `${seedId}.json`), 32 * 1024 * 1024);
  const source = owned(path.join(storage, 'seeds'), path.resolve(base, seed.directory));
  const pinned = async (file) => {
    const relative = path.relative(source, owned(source, file)).replaceAll('\\', '/');
    const entry = seed.files.find((f) => f.path.replaceAll('\\', '/') === relative);
    assert.ok(entry, 'PCM input is outside the retained seed');
    assert.equal(await fsp.realpath(file), file);
    assert.equal((await fsp.stat(file)).size, entry.size_bytes);
    assert.equal(await hash(file, remaining), entry.sha256);
    return file;
  };
  const config = await json(await pinned(path.join(source, 'tts-runtime.json')));
  const python = await pinned(config.python_executable);
  const site_packages = owned(
    source,
    path.join(path.dirname(path.dirname(python)), 'venv/Lib/site-packages'),
  );
  const library_files = seed.files
    .filter((f) =>
      /\/venv\/Lib\/site-packages\/(numpy|scipy|soundfile|_soundfile|_cffi_backend|cffi)/i.test(
        f.path.replaceAll('\\', '/'),
      ),
    )
    .map((f) => ({ path: owned(source, path.join(source, f.path)), sha256: f.sha256 }));
  for (const name of ['numpy', 'scipy', 'soundfile'])
    assert.ok(library_files.some((f) => path.relative(site_packages, f.path).startsWith(name)));
  const components = await json(path.join(storage, 'wheels', componentId, 'components.json'));
  const wheel = components.files.find(
    (f) => path.basename(f.path) === 'puza_tts-0.3.0-py3-none-any.whl',
  );
  assert.ok(wheel);
  const wheelPath = owned(path.join(storage, 'wheels'), path.resolve(base, wheel.path));
  assert.equal(await fsp.realpath(wheelPath), wheelPath);
  assert.equal((await fsp.stat(wheelPath)).size, wheel.size_bytes);
  assert.equal(await hash(wheelPath, remaining), wheel.sha256);
  const profile = path.join(run.directory, 'profile');
  await fsp.mkdir(profile);
  const descriptor = path.join(run.directory, 'inputs.json');
  await fsp.writeFile(
    descriptor,
    JSON.stringify({ site_packages, library_files, wheel: wheelPath }),
    { flag: 'wx' },
  );
  admission = await acquireDownloadAdmission();
  log = await fsp.open(path.join(run.directory, 'pcm.log'), 'wx');
  try {
    await runInstaller(
      python,
      ['-I', '-S', '-B', path.join(root, 'tools/ui-e2e/pcm-regression.py'), descriptor, profile],
      {
        windowsHide: true,
        stdio: ['ignore', log.fd, log.fd],
        env: { ...process.env, PYTHONHOME: '', PYTHONPATH: '', PYTHONDONTWRITEBYTECODE: '1' },
      },
      remaining,
    );
  } catch (error) {
    confirmed = !(error instanceof UnconfirmedCleanup);
    if (!confirmed) throw error;
    throw new Error(`Read-only PCM regression failed; inspect pcm.log: ${error.message}`);
  }
  const result = await json(path.join(profile, 'validation.json'));
  assert.equal(result.proof.models_loaded, false);
  return { ...result, wheel_sha256: wheel.sha256, seed_id: seedId };
});
await report.step('cleanup', async () => {
  await log?.close();
  if (!confirmed || report.data.cleanup_unconfirmed)
    throw new UnconfirmedCleanup('PCM regression child or admission release is unconfirmed');
  await requireQuiescent();
  await admission?.();
  await release?.(true);
  return { confirmed: true };
});
await report.save();
process.stdout.write(`PCM report: ${run.id}\n`);
process.exitCode = report.exitCode();
