import fsp from 'node:fs/promises';
import path from 'node:path';
import { runInstaller, installerBudget } from './installer.mjs';
import { root, storage, owned, json, hash } from './core.mjs';

export async function installComponents(
  profile,
  wheels,
  remaining = () => 300000,
  signal,
  onInstaller,
) {
  if (!Array.isArray(wheels) || wheels.length !== 2) throw new Error('Pin both component wheels');
  const names = new Set([
    'puza_tts-0.3.0-py3-none-any.whl',
    'voxscript_contract-0.2.0-py3-none-any.whl',
  ]);
  const inputs = [];
  for (const item of wheels) {
    remaining();
    const file = owned(path.join(storage, 'wheels'), path.resolve(root, item.path));
    if (
      !names.delete(path.basename(file)) ||
      (await fsp.realpath(file)) !== file ||
      (await fsp.stat(file)).size !== item.size_bytes ||
      (await hash(file, remaining)) !== item.sha256
    )
      throw new Error('Pinned component wheel differs');
    inputs.push(file);
  }
  const configFile = path.join(profile, 'tts-runtime.json');
  const config = await json(configFile);
  const inventory = await json(path.join(config.package_root, 'integrity.json'));
  const directory = owned(
    profile,
    path.join(profile, 'tts/packages', wheels.map((item) => item.sha256.slice(0, 12)).join('-')),
  );
  if (
    await fsp.access(directory).then(
      () => true,
      () => false,
    )
  )
    throw new Error('Component target already exists');
  installerBudget(remaining, signal);
  const log = await fsp.open(path.join(profile, 'component-install.log'), 'wx');
  try {
    await runInstaller(
      config.python_executable,
      [
        '-I',
        '-B',
        '-m',
        'pip',
        'install',
        '--no-index',
        '--no-deps',
        '--no-compile',
        '--target',
        directory,
        ...inputs,
      ],
      {
        windowsHide: true,
        stdio: ['ignore', log.fd, log.fd],
        env: {
          ...process.env,
          PYTHONHOME: '',
          PYTHONPATH: '',
          PIP_NO_INDEX: '1',
          PYTHONDONTWRITEBYTECODE: '1',
        },
      },
      remaining,
      signal,
      onInstaller,
    );
  } finally {
    await log.close();
  }
  const files = [];
  async function scan(current) {
    for (const entry of await fsp.readdir(current, { withFileTypes: true })) {
      remaining();
      if (entry.name === '__pycache__' || entry.name.endsWith('.pyc')) continue;
      const file = path.join(current, entry.name);
      if (entry.isSymbolicLink()) throw new Error('Component package contains redirected files');
      if (entry.isDirectory()) await scan(file);
      else
        files.push({ path: path.relative(directory, file), sha256: await hash(file, remaining) });
      if (files.length > 4096) throw new Error('Component package exceeds its file budget');
    }
  }
  await scan(directory);
  inventory.package_files = files;
  const inventoryFile = path.join(directory, 'integrity.json');
  await fsp.writeFile(inventoryFile, JSON.stringify(inventory, null, 2));
  config.package_root = directory;
  config.profile_root = path.join(directory, 'share/puza-tts/profiles');
  config.integrity_sha256 = await hash(inventoryFile);
  await fsp.writeFile(configFile, JSON.stringify(config, null, 2));
  return {
    wheels,
    package_files: files.length,
    integrity_sha256: config.integrity_sha256,
    models_loaded: false,
    voices_modified: false,
  };
}
