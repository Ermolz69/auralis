import fs from 'node:fs';
import fsp from 'node:fs/promises';
import path from 'node:path';

const SELECTION_TIMEOUT_MS = 900_000;
const POLL_INTERVAL_MS = 200;
const RELEASE_PATH = 'modules/auralis-translate/models/releases/hy_mt2_1_8b_q4_k_m.windows_x64_cpu.experimental.json';

export async function seedPackageCache(rootDir, dataRoot, sourceDirectory) {
  if (!sourceDirectory || !path.isAbsolute(sourceDirectory)) {
    throw new Error('Package native E2E requires an absolute AURALIS_TEST_PACKAGE_ASSET_DIR');
  }
  const release = await readRelease(rootDir);
  const variant = release.runtime.variants.find((item) => item.backend === 'cpu');
  if (!variant) throw new Error('Pinned release does not declare a CPU package');
  const assets = [
    release.model.file,
    release.model.license.notice,
    release.runtime.license.notice,
    variant.archive,
  ];
  const cache = path.join(dataRoot, 'cache', 'translation-downloads');
  await fsp.mkdir(cache, { recursive: true });
  for (const asset of assets) {
    const source = path.join(sourceDirectory, asset.filename);
    const metadata = await fsp.stat(source);
    if (!metadata.isFile() || metadata.size !== asset.bytes) {
      throw new Error(`Pinned package cache has a missing or incomplete ${asset.filename}`);
    }
    await fsp.copyFile(source, path.join(cache, asset.filename), fs.constants.COPYFILE_EXCL);
  }
  return release;
}

export async function waitForPackageSelection(dataRoot, app) {
  const deadline = Date.now() + SELECTION_TIMEOUT_MS;
  const checkpointPath = path.join(dataRoot, 'native-e2e-bootstrap.txt');
  while (Date.now() < deadline) {
    if (app.child.exitCode !== null || app.child.signalCode !== null) {
      throw new Error(`Native application exited before package selection: ${app.child.exitCode}`);
    }
    const checkpoints = await fsp.readFile(checkpointPath, 'utf8').catch((error) => {
      if (error.code === 'ENOENT') return '';
      throw error;
    });
    if (checkpoints.split(/\r?\n/).some((line) => line.startsWith('scenario-failed:'))) {
      throw new Error('Native package selection failed in the React runner');
    }
    if (checkpoints.split(/\r?\n/).includes('translation-package-selected')) return;
    await delay(POLL_INTERVAL_MS);
  }
  throw new Error('Timed out waiting for native package installation and selection');
}

export async function verifyInstalledPackage(dataRoot, release) {
  const marker = JSON.parse(
    await fsp.readFile(path.join(dataRoot, 'translation-package-selection.json'), 'utf8'),
  );
  if (marker.schema_version !== 1 || marker.release_id !== release.id || marker.backend !== 'cpu') {
    throw new Error('Native package selection marker does not match the pinned release');
  }
  const packageRoot = path.join(dataRoot, 'translation-packages', release.id, 'cpu');
  const files = [
    'release.json',
    'profile.json',
    path.join('runtime', 'llama-server.exe'),
    path.join('model', release.model.file.filename),
    path.join('notices', release.model.license.notice.filename),
    path.join('notices', release.runtime.license.notice.filename),
    path.join('assets', release.runtime.variants.find((item) => item.backend === 'cpu').archive.filename),
  ];
  for (const file of files) {
    if (!(await fsp.stat(path.join(packageRoot, file))).isFile()) {
      throw new Error(`Native package is missing ${file}`);
    }
  }
  if (fs.existsSync(path.join(dataRoot, 'translation-runtime.json'))) {
    throw new Error('Package E2E unexpectedly used the manual runtime configuration');
  }
}

async function readRelease(rootDir) {
  return JSON.parse(await fsp.readFile(path.join(rootDir, RELEASE_PATH), 'utf8'));
}

function delay(durationMs) {
  return new Promise((resolve) => setTimeout(resolve, durationMs));
}
