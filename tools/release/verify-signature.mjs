import childProcess from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const currentFile = fileURLToPath(import.meta.url);
const rootDir = path.resolve(path.dirname(currentFile), '../..');
const bundleRoot = path.join(rootDir, 'target/release/bundle');

try {
  if (process.platform === 'win32') verifyWindows();
  else if (process.platform === 'darwin') verifyMacos();
  else process.stdout.write('No mandatory native signature check configured for Linux.\n');
} catch (error) {
  process.stderr.write(`${error.message}\n`);
  process.exitCode = 1;
}

function verifyWindows() {
  const bundles = walk(bundleRoot).filter(
    (file) => file.endsWith('.msi') || file.toLowerCase().endsWith('-setup.exe'),
  );
  if (bundles.length < 2) throw new Error('Signed MSI and NSIS bundles are required');
  for (const bundle of bundles) {
    const script = `(Get-AuthenticodeSignature -LiteralPath '${escapePowerShell(bundle)}').Status`;
    const result = run('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', script]);
    if (result.stdout.trim() !== 'Valid') {
      throw new Error(`Authenticode signature is not valid for ${path.basename(bundle)}`);
    }
  }
}

function verifyMacos() {
  const app = unique(walk(bundleRoot).filter((entry) => entry.endsWith('.app')), 'macOS app bundle');
  const dmg = unique(walk(bundleRoot).filter((entry) => entry.endsWith('.dmg')), 'notarized DMG');
  run('codesign', ['--verify', '--deep', '--strict', '--verbose=2', app]);
  run('spctl', ['--assess', '--type', 'execute', '--verbose=2', app]);
  run('xcrun', ['stapler', 'validate', dmg]);
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

function unique(entries, label) {
  if (entries.length !== 1) throw new Error(`Expected exactly one ${label}, found ${entries.length}`);
  return entries[0];
}

function run(command, args) {
  const result = childProcess.spawnSync(command, args, {
    encoding: 'utf8',
    timeout: 180_000,
    windowsHide: true,
  });
  if (result.error || result.status !== 0) {
    const detail =
      result.error?.message ||
      result.stderr?.trim() ||
      result.stdout?.trim() ||
      `exit code ${result.status}`;
    throw new Error(`Signature verification failed: ${detail}`);
  }
  return result;
}

function escapePowerShell(value) {
  return value.replaceAll("'", "''");
}
