import assert from 'node:assert/strict';
import fsp from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { test } from 'node:test';
import { buildIdentity, buildPaths } from './build-identity.mjs';
import { owned } from './core.mjs';
test('build identity includes package and platform inputs and rejects a dirty dependency', async () => {
  const base = await fsp.mkdtemp(path.join(os.tmpdir(), 'auralis-ui-build-'));
  const git = (cwd, args) => execFileSync('git', args, { cwd, windowsHide: true, stdio: 'pipe' });
  const commit = (cwd) => {
    git(cwd, ['add', '.']);
    git(cwd, [
      '-c',
      'user.name=Fixture',
      '-c',
      'user.email=fixture@example.invalid',
      'commit',
      '--allow-empty',
      '-m',
      'fixture',
    ]);
  };
  try {
    const dependency = path.join(base, 'modules/auralis-translate');
    await fsp.mkdir(dependency, { recursive: true });
    git(base, ['init']);
    await fsp.writeFile(path.join(base, 'package.json'), '{}');
    commit(base);
    git(dependency, ['init']);
    await fsp.writeFile(path.join(dependency, 'source.rs'), '// original');
    commit(dependency);
    const first = await buildIdentity(base);
    assert.ok(first.sources.some((f) => f.path === 'package.json'));
    for (const field of [
      'pnpm-lock.yaml',
      'src-tauri',
      'apps/desktop/index.html',
      'Taskfile.yml',
      'taskfiles',
      'tools/media-tools',
      'tools/ui-e2e',
    ])
      assert.ok(buildPaths.includes(field));
    await fsp.writeFile(path.join(dependency, 'source.rs'), '// changed');
    await assert.rejects(buildIdentity(base), /must be clean/);
  } finally {
    await fsp.rm(owned(os.tmpdir(), base), { recursive: true, force: true });
  }
});
