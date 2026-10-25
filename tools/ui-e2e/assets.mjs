import fsp from 'node:fs/promises';
import path from 'node:path';
import { root, storage, owned, hash, json, idPattern } from './core.mjs';

const id = process.argv[2];
const resume = process.argv[3] === 'resume';
if (!idPattern.test(id ?? '')) throw new Error('Supply a new component seed identity');
const destination = owned(path.join(storage, 'seeds'), path.join(storage, 'seeds', id));
const sourceRoot = path.join(process.env.APPDATA, 'com.auralis.desktop');
const config = await json(path.join(sourceRoot, 'tts-runtime.json'));
const inventory = await json(path.join(config.package_root, 'integrity.json'));
if ((await hash(path.join(config.package_root, 'integrity.json'))) !== config.integrity_sha256)
  throw new Error('Installed component inventory differs');
let total = 0;
let count = 0;
const deadline = Date.now() + 1800000;
async function copy(source, relative, expected) {
  const target = owned(destination, path.join(destination, relative));
  const stat = await fsp.stat(source);
  if (!stat.isFile()) throw new Error(`Expected component file: ${relative}`);
  total += stat.size;
  if (++count > 65536 || total > 32 * 1024 ** 3 || Date.now() > deadline)
    throw new Error('Component seed budget exceeded');
  const digest = await hash(source);
  if (expected && digest !== expected) throw new Error(`Component pin differs: ${relative}`);
  await fsp.mkdir(path.dirname(target), { recursive: true });
  const exists = await fsp.lstat(target).catch((error) => {
    if (error.code !== 'ENOENT') throw error;
    return null;
  });
  if (exists && !resume) throw new Error('Component target already exists');
  if (!exists)
    await fsp.copyFile(source, target, (await import('node:fs')).constants.COPYFILE_EXCL);
  if ((await hash(target)) !== digest) throw new Error(`Component copy differs: ${relative}`);
  if (count % 1000 === 0) process.stdout.write(`Copied ${count} files, ${total} bytes\n`);
}
async function tree(source, relative) {
  for (const item of await fsp.readdir(source, { withFileTypes: true })) {
    if (item.name === '__pycache__' || item.name.endsWith('.pyc')) continue;
    if (item.isSymbolicLink()) throw new Error('Runtime trees must not contain redirected files');
    const next = path.join(source, item.name);
    if (item.isDirectory()) await tree(next, path.join(relative, item.name));
    else if (item.isFile()) await copy(next, path.join(relative, item.name));
  }
}
{
  await fsp.mkdir(path.dirname(destination), { recursive: true });
  if (resume) {
    if ((await fsp.realpath(destination)) !== destination) throw new Error('Redirected seed');
    if (
      await fsp.access(path.join(storage, 'seeds', `${id}.json`)).then(
        () => true,
        () => false,
      )
    )
      throw new Error('Completed seeds cannot be resumed');
  } else await fsp.mkdir(destination, { recursive: false });
  const runtime = path.dirname(path.dirname(config.python_executable));
  const packageRelative = path.join('tts/packages', path.basename(config.package_root));
  const runtimeRelative = path.join('tts/runtime', path.basename(runtime));
  const voiceRelative = path.join('tts/voice-stores', path.basename(config.voice_store));
  await tree(runtime, runtimeRelative);
  await tree(config.package_root, packageRelative);
  await tree(config.voice_store, voiceRelative);
  for (const item of inventory.model_files)
    await copy(
      path.join(config.model_cache, item.path),
      path.join('tts/model-cache', item.path),
      item.sha256,
    );
  for (const item of inventory.package_files)
    if ((await hash(path.join(destination, packageRelative, item.path))) !== item.sha256)
      throw new Error('Installed package pin differs');
  for (const item of inventory.dependencies) {
    const relative = path.relative(runtime, owned(runtime, item.path));
    const target = path.join(destination, runtimeRelative, relative);
    if ((await hash(target)) !== item.sha256) throw new Error('Native dependency differs');
    item.path = target;
  }
  const inventoryFile = path.join(destination, packageRelative, 'integrity.json');
  await fsp.writeFile(inventoryFile, JSON.stringify(inventory, null, 2));
  const seeded = {
    ...config,
    python_executable: path.join(destination, runtimeRelative, 'base/python.exe'),
    package_root: path.join(destination, packageRelative),
    voice_store: path.join(destination, voiceRelative),
    profile_root: path.join(destination, packageRelative, 'share/puza-tts/profiles'),
    model_cache: path.join(destination, 'tts/model-cache'),
    integrity_sha256: await hash(inventoryFile),
  };
  await fsp.writeFile(path.join(destination, 'tts-runtime.json'), JSON.stringify(seeded, null, 2));
  const releaseManifest = await json(
    path.join(
      root,
      'modules/auralis-translate/models/releases/hy_mt2_1_8b_q4_k_m.windows_x64_cpu.experimental.json',
    ),
  );
  const downloadRoot = path.join(path.dirname(root), 'auralis-translate/.cache/install-sources');
  for (const asset of [
    releaseManifest.model.file,
    releaseManifest.model.license.notice,
    releaseManifest.runtime.license.notice,
    releaseManifest.runtime.variants[0].archive,
  ])
    await copy(
      path.join(downloadRoot, asset.filename),
      path.join('cache/translation-downloads', asset.filename),
      asset.sha256,
    );
  const files = [];
  async function scan(directory) {
    for (const item of await fsp.readdir(directory, { withFileTypes: true })) {
      const file = path.join(directory, item.name);
      if (item.isDirectory()) await scan(file);
      else
        files.push({
          path: path.relative(destination, file).replaceAll('\\', '/'),
          size_bytes: (await fsp.stat(file)).size,
          sha256: await hash(file),
        });
    }
  }
  await scan(destination);
  const manifest = { directory: path.relative(root, destination).replaceAll('\\', '/'), files };
  await fsp.writeFile(
    path.join(storage, 'seeds', `${id}.json`),
    JSON.stringify(manifest, null, 2),
    { flag: 'wx' },
  );
  process.stdout.write(
    `Prepared ${files.length} component files; voice bytes and qualification preserved\n`,
  );
}
