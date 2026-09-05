import childProcess from 'node:child_process';
import fs from 'node:fs';
import fsp from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyMaterializedTree } from '../media-tools/bundle-verify.mjs';
import { loadManifest, resolveTarget } from '../media-tools/manifest.mjs';

export function findUnique(root, predicate, label) {
  const matches = walk(root).filter(predicate);
  if (matches.length !== 1) {
    throw new Error(`Expected exactly one ${label}, found ${matches.length} under ${root}`);
  }
  return matches[0];
}

export async function assertProcessStaysAlive(executable, durationMs = 8_000) {
  const child = childProcess.spawn(executable, [], {
    cwd: path.dirname(executable),
    env: { ...process.env, AURALIS_OBSERVABILITY_ENABLED: 'false' },
    stdio: 'ignore',
    windowsHide: true,
  });
  try {
    const exit = new Promise((resolve) => child.once('exit', (code, signal) => resolve({ code, signal })));
    const spawnError = new Promise((resolve) => child.once('error', (error) => resolve({ error })));
    const outcome = await Promise.race([
      exit,
      spawnError,
      new Promise((resolve) => setTimeout(() => resolve(null), durationMs)),
    ]);
    if (outcome) {
      if (outcome.error) throw new Error(`Installed application could not start: ${outcome.error.message}`);
      throw new Error(
        `Installed application exited during launch smoke: code ${outcome.code}, signal ${outcome.signal}`,
      );
    }
  } finally {
    await terminate(child);
  }
}

export async function runInstallLaunchSmoke(rootDir) {
  if (process.platform === 'win32') return smokeWindows(rootDir);
  if (process.platform === 'darwin') return smokeMacos(rootDir);
  throw new Error(`Install/launch smoke is not configured for ${process.platform}`);
}

async function smokeWindows(rootDir) {
  const bundleRoot = path.join(rootDir, 'target/release/bundle');
  const installer = findUnique(
    bundleRoot,
    (file) => file.toLowerCase().endsWith('-setup.exe'),
    'NSIS installer',
  );
  const installRoot = await fsp.mkdtemp(path.join(os.tmpdir(), 'auralis-installed-'));
  let installedBySmoke = false;
  try {
    run(installer, ['/S', `/D=${installRoot}`], [0]);
    installedBySmoke = true;
    const executable = findUnique(
      installRoot,
      (file) => file.toLowerCase().endsWith('auralis-app.exe'),
      'installed Auralis executable',
    );
    await verifyInstalledMediaTools(rootDir, installRoot);
    if (process.env.AURALIS_REQUIRE_SIGNING === 'true') {
      verifyInstalledWindowsSignatures(installRoot);
    }
    await assertProcessStaysAlive(executable);
  } finally {
    if (installedBySmoke) {
      const uninstaller = findUnique(
        installRoot,
        (file) => path.basename(file).toLowerCase() === 'uninstall.exe',
        'NSIS uninstaller',
      );
      run(uninstaller, ['/S'], [0]);
    }
    await fsp.rm(installRoot, { recursive: true, force: true });
  }
}

async function smokeMacos(rootDir) {
  const bundleRoot = path.join(rootDir, 'target/release/bundle');
  const dmg = findUnique(bundleRoot, (entry) => entry.endsWith('.dmg'), 'macOS DMG');
  const mountRoot = await fsp.mkdtemp(path.join(os.tmpdir(), 'auralis-mounted-'));
  const installRoot = await fsp.mkdtemp(path.join(os.tmpdir(), 'auralis-installed-'));
  let mounted = false;
  try {
    run('hdiutil', ['attach', '-nobrowse', '-readonly', '-mountpoint', mountRoot, dmg], [0]);
    mounted = true;
    const sourceApp = findUnique(mountRoot, (entry) => entry.endsWith('.app'), 'mounted app bundle');
    const installedApp = path.join(installRoot, path.basename(sourceApp));
    await fsp.cp(sourceApp, installedApp, { recursive: true, preserveTimestamps: true });
    const executable = findUnique(
      path.join(installedApp, 'Contents/MacOS'),
      (file) => fs.statSync(file).isFile(),
      'installed macOS executable',
    );
    await verifyInstalledMediaTools(rootDir, installedApp);
    await assertProcessStaysAlive(executable);
  } finally {
    if (mounted) run('hdiutil', ['detach', mountRoot], [0]);
    await fsp.rm(mountRoot, { recursive: true, force: true });
    await fsp.rm(installRoot, { recursive: true, force: true });
  }
}

async function verifyInstalledMediaTools(rootDir, installedRoot) {
  const manifest = loadManifest(path.join(rootDir, 'tools/media-tools/manifest.json'));
  await verifyMaterializedTree(installedRoot, manifest, resolveTarget());
}

function verifyInstalledWindowsSignatures(installRoot) {
  const executables = walk(installRoot).filter((file) => file.toLowerCase().endsWith('.exe'));
  if (executables.length < 5) {
    throw new Error(`Expected signed app, uninstaller and media tools; found ${executables.length}`);
  }
  for (const executable of executables) {
    const escaped = executable.replaceAll("'", "''");
    const result = run(
      'powershell.exe',
      ['-NoProfile', '-NonInteractive', '-Command', `(Get-AuthenticodeSignature -LiteralPath '${escaped}').Status`],
      [0],
    );
    if (result.stdout.trim() !== 'Valid') {
      throw new Error(`Installed executable is not Authenticode-signed: ${path.basename(executable)}`);
    }
  }
}

function walk(root, entries = []) {
  if (!fs.existsSync(root)) return entries;
  for (const entry of fs.readdirSync(root, { withFileTypes: true })) {
    const candidate = path.join(root, entry.name);
    entries.push(candidate);
    if (entry.isDirectory()) walk(candidate, entries);
  }
  return entries;
}

function run(command, args, successfulCodes) {
  const result = childProcess.spawnSync(command, args, {
    encoding: 'utf8',
    timeout: 180_000,
    windowsHide: true,
  });
  if (result.error || !successfulCodes.includes(result.status)) {
    const detail =
      result.error?.message ||
      result.stderr?.trim() ||
      result.stdout?.trim() ||
      `exit code ${result.status ?? 'unknown'}`;
    throw new Error(`${command} failed: ${detail}`);
  }
  return result;
}

async function terminate(child) {
  if (child.exitCode !== null || child.signalCode !== null) return;
  if (process.platform === 'win32') {
    childProcess.spawnSync('taskkill.exe', ['/pid', String(child.pid), '/t', '/f'], {
      stdio: 'ignore',
      windowsHide: true,
    });
  } else {
    child.kill('SIGTERM');
  }
  await Promise.race([
    new Promise((resolve) => child.once('exit', resolve)),
    new Promise((resolve) => setTimeout(resolve, 5_000)),
  ]);
  if (child.exitCode === null && child.signalCode === null) child.kill('SIGKILL');
}

const currentFile = fileURLToPath(import.meta.url);
if (path.resolve(process.argv[1] ?? '') === currentFile) {
  const rootDir = path.resolve(path.dirname(currentFile), '../..');
  runInstallLaunchSmoke(rootDir)
    .then(() => process.stdout.write(`Installed application launch smoke passed on ${process.platform}.\n`))
    .catch((error) => {
      process.stderr.write(`${error.message}\n`);
      process.exitCode = 1;
    });
}
