import fsp from 'node:fs/promises';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { root, storage, hash, json } from './core.mjs';

if (process.platform !== 'win32' || process.arch !== 'x64')
  throw new Error('The initial native candidate supports Windows x64');
const source = path.resolve(
  root,
  process.env.CARGO_TARGET_DIR ?? 'target',
  'release',
  'auralis-app.exe',
);
const git = (args) => execFileSync('git', args, { cwd: root, encoding: 'utf8' }).trim();
const files = execFileSync(
  'git',
  [
    'ls-files',
    '-z',
    '--cached',
    '--others',
    '--exclude-standard',
    '--',
    'src-tauri/src',
    'src-tauri/Cargo.toml',
    'src-tauri/tauri.conf.json',
    'apps/desktop/src',
    'apps/desktop/package.json',
    'apps/desktop/vite.config.ts',
    'crates',
    'Cargo.toml',
    'Cargo.lock',
  ],
  { cwd: root, encoding: 'utf8' },
)
  .split('\0')
  .filter(Boolean)
  .sort();
const sources = [];
for (const file of files) sources.push({ path: file, sha256: await hash(path.join(root, file)) });
const input = {
  sources,
  commit: git(['rev-parse', 'HEAD']),
  translate_commit: git(['-C', 'modules/auralis-translate', 'rev-parse', 'HEAD']),
};
const inputFile = path.join(storage, 'build-input.json');
if (process.argv[2] === 'begin') {
  await fsp.mkdir(storage, { recursive: true });
  await fsp.writeFile(inputFile, JSON.stringify(input));
  process.stdout.write(`Frozen ${sources.length} application source files\n`);
  process.exit(0);
}
if (JSON.stringify(await json(inputFile)) !== JSON.stringify(input))
  throw new Error('Application sources changed during the candidate build');
const sha256 = await hash(source);
const directory = path.join(storage, 'candidates', sha256);
await fsp.mkdir(path.dirname(directory), { recursive: true });
const existing = await fsp
  .stat(directory)
  .then(() => true)
  .catch((error) => {
    if (error.code === 'ENOENT') return false;
    throw error;
  });
if (existing) {
  const receipt = await json(path.join(directory, 'candidate.json'));
  if (
    receipt.version !== 2 ||
    JSON.stringify(receipt.sources) !== JSON.stringify(sources) ||
    (await hash(path.join(directory, 'auralis-app.exe'))) !== sha256
  )
    throw new Error('Retained candidate identity differs');
  for (const resource of receipt.resources)
    if ((await hash(path.join(directory, resource.path))) !== resource.sha256)
      throw new Error('Retained candidate resource changed');
  await fsp.writeFile(path.join(storage, 'candidate.json'), JSON.stringify({ sha256 }));
  process.stdout.write(`Reused verified candidate ${sha256}\n`);
  process.exit(0);
}
await fsp.mkdir(directory);
await fsp.copyFile(source, path.join(directory, 'auralis-app.exe'));
await fsp.cp(path.join(root, 'src-tauri/binaries'), path.join(directory, 'binaries'), {
  recursive: true,
  errorOnExist: true,
  force: false,
});
const resources = [];
for (const name of (await fsp.readdir(path.join(directory, 'binaries'))).sort()) {
  const file = path.join(directory, 'binaries', name);
  if ((await fsp.lstat(file)).isFile())
    resources.push({ path: `binaries/${name}`, sha256: await hash(file) });
}
const candidate = {
  version: 2,
  sources,
  sha256,
  commit: git(['rev-parse', 'HEAD']),
  translate_commit: git(['-C', 'modules/auralis-translate', 'rev-parse', 'HEAD']),
  working_diff_sha256: (await import('node:crypto'))
    .createHash('sha256')
    .update(
      execFileSync(
        'git',
        ['diff', 'HEAD', '--', 'src-tauri', 'apps/desktop', 'crates', 'Cargo.lock', 'Cargo.toml'],
        { cwd: root },
      ),
    )
    .digest('hex'),
  resources,
};
await fsp.writeFile(path.join(directory, 'candidate.json'), JSON.stringify(candidate, null, 2), {
  flag: 'wx',
});
await fsp.writeFile(path.join(storage, 'candidate.json'), JSON.stringify({ sha256 }, null, 2));
process.stdout.write(`Retained native release candidate ${sha256}\n`);
