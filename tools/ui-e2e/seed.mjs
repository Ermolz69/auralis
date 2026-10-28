import fsp from 'node:fs/promises';
import path from 'node:path';
import { root, storage, owned, hash, Blocked } from './core.mjs';
import { relocateTts } from './relocate.mjs';
import { installComponents } from './components.mjs';
import fs from 'node:fs';
import { pipeline } from 'node:stream/promises';

export function validateSeedPath(value) {
  if (typeof value !== 'string' || path.isAbsolute(value))
    throw new Error('Seed files must have relative component paths');
  const parts = value.replaceAll('\\', '/').split('/');
  if (parts.some((part) => !part || part === '.' || part === '..' || part.includes(':')))
    throw new Error('Seed paths cannot traverse directories');
  const configs = new Set([
    'tts-runtime.json',
    'translation-runtime.json',
    'translation-package-selection.json',
    'caster-runtime.json',
    'marker-runtime.json',
    'speech-limits.json',
  ]);
  const assets = new Set([
    'tts',
    'translation-packages',
    'runtime',
    'models',
    'voices',
    'profiles',
    'assets',
    'model-cache',
    'caster',
    'marker',
  ]);
  if (parts[0] === 'cache' && parts[1] === 'translation-downloads' && parts.length === 3) return;
  if (
    parts.length === 1 ? !configs.has(parts[0].toLowerCase()) : !assets.has(parts[0].toLowerCase())
  )
    throw new Error('Seed application databases, jobs, projects and WebView2 state are prohibited');
}

export async function seedProfile(spec, run, remaining = () => 300000, signal) {
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
    remaining();
    validateSeedPath(item.path);
    if (seen.has(item.path) || !Number.isSafeInteger(item.size_bytes) || item.size_bytes < 0)
      throw new Error('Invalid or duplicated seed file');
    seen.add(item.path);
    bytes += item.size_bytes;
    if (bytes > 32 * 1024 ** 3) throw new Error('Seed exceeds the acquisition budget');
    const source = owned(directory, path.join(directory, item.path));
    if ((await fsp.realpath(source)) !== source)
      throw new Error('Seed file must not be redirected');
    if ((await fsp.stat(source)).size !== item.size_bytes || (await hash(source, remaining, signal)) !== item.sha256)
      throw new Blocked(`Seed changed: ${item.path}`);
    const target = owned(destination, path.join(destination, item.path));
    await fsp.mkdir(path.dirname(target), { recursive: true });
    await pipeline(fs.createReadStream(source), fs.createWriteStream(target, { flags: 'wx' }), { signal });
    remaining();
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
  const relocated = seen.has('tts-runtime.json') ? await relocateTts(destination, directory) : null;
  const components = spec.component_wheels
    ? await installComponents(destination, spec.component_wheels, remaining)
    : null;
  return {
    seeded: true,
    files: seed.files.length,
    bytes,
    qualification_modified: false,
    relocated,
    components,
  };
}
