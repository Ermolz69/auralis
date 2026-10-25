import fsp from 'node:fs/promises';
import path from 'node:path';
import { hash, owned, json } from './core.mjs';

export async function relocateTts(destination, source) {
  const file = path.join(destination, 'tts-runtime.json');
  const config = await json(file);
  const inventoryFile = owned(destination, path.join(config.package_root, 'integrity.json'));
  const inventory = await json(inventoryFile);
  for (const dependency of inventory.dependencies) {
    dependency.path = owned(
      destination,
      path.join(destination, path.relative(source, owned(source, dependency.path))),
    );
    if ((await hash(dependency.path)) !== dependency.sha256)
      throw new Error('Relocated dependency changed');
  }
  await fsp.writeFile(inventoryFile, JSON.stringify(inventory, null, 2));
  config.integrity_sha256 = await hash(inventoryFile);
  await fsp.writeFile(file, JSON.stringify(config, null, 2));
  const runtime = path.dirname(path.dirname(config.python_executable));
  const venv = owned(destination, path.join(runtime, 'venv/pyvenv.cfg'));
  const contents = await fsp.readFile(venv, 'utf8');
  await fsp.writeFile(
    venv,
    contents.replace(/^home\s*=.*$/im, `home = ${path.join(runtime, 'base')}`),
  );
  return { inventory_sha256: config.integrity_sha256 };
}
