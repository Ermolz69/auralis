import fsp from 'node:fs/promises';
import path from 'node:path';
import { root, runs, owned, json } from './core.mjs';
import { runInstaller } from './installer.mjs';
import { requireQuiescent } from './processes.mjs';

const profile = owned(runs, path.resolve(process.argv[2]));
const config = await json(path.join(profile, 'tts-runtime.json'));
await requireQuiescent();
const file = path.join(path.dirname(profile), `metadata-diagnostics-${Date.now()}.log`);
const log = await fsp.open(file, 'wx');
try {
  await runInstaller(
    config.python_executable,
    ['-I', '-B', path.join(root, 'tools/ui-e2e/project-diagnostics.py'), profile],
    {
      windowsHide: true,
      stdio: ['ignore', log.fd, log.fd],
    },
    () => 120000,
  );
} finally {
  await log.close();
  process.stdout.write(await fsp.readFile(file, 'utf8'));
}
