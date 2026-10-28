import path from 'node:path';
import fsp from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { hash, root } from './core.mjs';

export const buildPaths = [
  'package.json', 'pnpm-lock.yaml', 'pnpm-workspace.yaml', 'Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml',
  'src-tauri', 'crates', 'vendor', 'apps/desktop/src', 'apps/desktop/public', 'apps/desktop/build',
  'apps/desktop/index.html', 'apps/desktop/package.json', 'apps/desktop/vite.config.ts',
  'apps/desktop/tsconfig.json', 'apps/desktop/tsconfig.app.json', 'apps/desktop/tsconfig.node.json',
];

export async function buildIdentity(base = root) {
  const git = (args, cwd = base) => execFileSync('git', args, {cwd,encoding:'utf8',windowsHide:true}).trim();
  const dependency = path.join(base,'modules/auralis-translate');
  const actual = await fsp.realpath(git(['rev-parse','--show-toplevel'],dependency));
  if (actual.toLowerCase() !== dependency.toLowerCase()) throw new Error('Initialize the pinned Translate submodule before building');
  if (git(['status','--porcelain','--untracked-files=normal'],dependency)) throw new Error('Translate build dependency must be clean');
  const list = (cwd, paths) => execFileSync('git',['ls-files','-z','--cached','--others','--exclude-standard','--',...paths], {cwd,encoding:'utf8',windowsHide:true}).split('\0').filter(Boolean).sort();
  const sources = [];
  for (const file of list(base,buildPaths)) sources.push({path:file,sha256:await hash(path.join(base,file))});
  const dependency_sources = [];
  for (const file of list(dependency,[])) dependency_sources.push({path:file,sha256:await hash(path.join(dependency,file))});
  return { sources, dependency_sources, commit:git(['rev-parse','HEAD']), translate_commit:git(['rev-parse','HEAD'],dependency), node:process.version };
}
