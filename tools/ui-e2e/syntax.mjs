import fsp from 'node:fs/promises';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { root } from './core.mjs';
for (const directory of ['tools/ui-e2e', 'apps/desktop/scripts']) {
  for (const name of await fsp.readdir(path.join(root, directory))) {
    if (!name.endsWith('.mjs') || (directory.endsWith('scripts') && !name.startsWith('ui-')))
      continue;
    execFileSync(process.execPath, ['--check', path.join(root, directory, name)], {
      stdio: 'inherit',
      windowsHide: true,
    });
  }
}
