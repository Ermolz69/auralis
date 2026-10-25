import fsp from 'node:fs/promises';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { root, storage, owned, json, hash, idPattern, UnconfirmedCleanup } from './core.mjs';
import { installComponents } from './components.mjs';
import { lockSession } from './session.mjs';

const [id, seedId, componentId] = process.argv.slice(2);
if (![id, seedId, componentId].every((value) => idPattern.test(value ?? '')))
  throw new Error('Pin probe, seed and component identities');
const directory = owned(storage, path.join(storage, 'component-checks', id));
await fsp.mkdir(path.dirname(directory), { recursive: true });
await fsp.mkdir(directory);
const seed = await json(path.join(storage, 'seeds', `${seedId}.json`), 32 * 1024 * 1024);
const source = owned(path.join(storage, 'seeds'), path.resolve(root, seed.directory));
const configFile = path.join(source, 'tts-runtime.json');
const config = await json(configFile);
for (const file of [
  configFile,
  config.python_executable,
  path.join(config.package_root, 'integrity.json'),
]) {
  const entry = seed.files.find(
    (item) => item.path.replaceAll('\\', '/') === path.relative(source, file).replaceAll('\\', '/'),
  );
  if (!entry || (await hash(file)) !== entry.sha256)
    throw new Error('Component probe input differs from the retained seed');
}
const release = await lockSession();
let confirmed = true;
const report = {
  version: 1,
  scope: 'component-model-free',
  models_loaded: false,
  voices_modified: false,
};
try {
  await fsp.writeFile(path.join(directory, 'tts-runtime.json'), JSON.stringify(config, null, 2));
  const components = await json(path.join(storage, 'wheels', componentId, 'components.json'));
  report.installation = await installComponents(directory, components.files);
  const configured = await json(path.join(directory, 'tts-runtime.json'));
  const log = await fsp.open(path.join(directory, 'probe.log'), 'wx');
  try {
    const child = spawn(
      configured.python_executable,
      [
        '-I',
        '-B',
        path.join(root, 'tools/ui-e2e/component-probe.py'),
        path.join(directory, 'tts-runtime.json'),
        path.join(directory, 'validation.json'),
      ],
      { windowsHide: true, stdio: ['ignore', log.fd, log.fd] },
    );
    await new Promise((resolve, reject) => {
      let exceeded = false;
      let releaseTimer;
      const timer = setTimeout(() => {
        exceeded = true;
        child.kill();
        releaseTimer = setTimeout(
          () => reject(new UnconfirmedCleanup('Component metadata probe exit is unconfirmed')),
          8000,
        );
      }, 60000);
      child.on('error', (error) => {
        clearTimeout(timer);
        clearTimeout(releaseTimer);
        reject(error);
      });
      child.on('exit', (code) => {
        clearTimeout(timer);
        clearTimeout(releaseTimer);
        code === 0 && !exceeded
          ? resolve()
          : reject(new Error('Installed component metadata validation failed'));
      });
    });
  } finally {
    await log.close();
  }
  report.validation = await json(path.join(directory, 'validation.json'));
  report.status = 'passed';
} catch (error) {
  confirmed = !(error instanceof UnconfirmedCleanup);
  report.status = 'failed';
  report.error = String(error.message);
  throw error;
} finally {
  await fsp.writeFile(path.join(directory, 'report.json'), JSON.stringify(report, null, 2));
  await release(confirmed);
  process.stdout.write(`Component report: ${id} ${report.status}\n`);
}
