import fsp from 'node:fs/promises';
import path from 'node:path';
import crypto from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { root, hash } from './core.mjs';

export async function frontendIdentity() {
  const directory = path.join(root, 'apps/desktop/dist');
  const pending = [directory],
    files = [];
  while (pending.length) {
    const current = pending.pop();
    for (const item of await fsp.readdir(current, { withFileTypes: true })) {
      const target = path.join(current, item.name);
      if (item.isSymbolicLink())
        throw new Error('Production frontend must not contain redirected assets');
      if (item.isDirectory()) pending.push(target);
      else if (item.isFile())
        files.push({
          path: path.relative(directory, target).replaceAll('\\', '/'),
          sha256: await hash(target),
        });
      if (files.length + pending.length > 1024)
        throw new Error('Production frontend inventory exceeds its budget');
    }
  }
  files.sort((a, b) => a.path.localeCompare(b.path));
  let commit = null;
  try {
    commit = execFileSync('git', ['rev-parse', 'HEAD'], {
      cwd: root,
      encoding: 'utf8',
      windowsHide: true,
    }).trim();
  } catch {}
  return {
    source_commit: commit,
    frontend_sha256: crypto.createHash('sha256').update(JSON.stringify(files)).digest('hex'),
    files,
  };
}
