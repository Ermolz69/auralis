import fsp from 'node:fs/promises';
import path from 'node:path';
import { root, storage, owned, hash, Blocked } from './core.mjs';

export async function seedProfile(spec, run) {
  if (!spec.runtime_seed) return { seeded: false };
  const seed = spec.runtime_seed;
  const directory = owned(path.join(storage, 'seeds'), path.resolve(root, seed.directory));
  if ((await fsp.realpath(directory)) !== directory)
    throw new Error('Seed directory must not be redirected');
  if (!Array.isArray(seed.files) || seed.files.length > 65536)
    throw new Error('Invalid bounded seed file manifest');
  const destination = path.join(run.directory, 'profile');
  let bytes = 0;
  const seen = new Set();
  for (const item of seed.files) {
    if (seen.has(item.path) || !Number.isSafeInteger(item.size_bytes) || item.size_bytes < 0)
      throw new Error('Invalid or duplicated seed file');
    seen.add(item.path);
    bytes += item.size_bytes;
    if (bytes > 32 * 1024 ** 3) throw new Error('Seed exceeds the acquisition budget');
    const source = owned(directory, path.join(directory, item.path));
    if ((await fsp.realpath(source)) !== source)
      throw new Error('Seed file must not be redirected');
    if ((await fsp.stat(source)).size !== item.size_bytes || (await hash(source)) !== item.sha256)
      throw new Blocked(`Seed changed: ${item.path}`);
    const target = owned(destination, path.join(destination, item.path));
    await fsp.mkdir(path.dirname(target), { recursive: true });
    await fsp.copyFile(source, target, (await import('node:fs')).constants.COPYFILE_EXCL);
  }
  const remap = (value) => {
    if (Array.isArray(value)) return value.map(remap);
    if (value && typeof value === 'object')
      return Object.fromEntries(Object.entries(value).map(([key, v]) => [key, remap(v)]));
    if (typeof value === 'string' && path.isAbsolute(value))
      return owned(
        destination,
        path.join(destination, path.relative(directory, owned(directory, value))),
      );
    return value;
  };
  for (const name of [
    'tts-runtime.json',
    'translation-runtime.json',
    'translation-package-selection.json',
  ]) {
    if (!seen.has(name)) continue;
    const file = path.join(destination, name);
    await fsp.writeFile(
      file,
      JSON.stringify(remap(JSON.parse(await fsp.readFile(file, 'utf8'))), null, 2),
    );
  }
  return { seeded: true, files: seed.files.length, bytes, qualification_modified: false };
}
