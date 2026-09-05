import childProcess from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { windowsSigningConfig } from './signing-config.mjs';

const target = process.argv[2];
if (!target) {
  process.stderr.write('Tauri did not provide a Windows signing target\n');
  process.exitCode = 1;
} else if (!process.env.WINDOWS_SIGNING_MODE && process.env.AURALIS_REQUIRE_SIGNING !== 'true') {
  process.stdout.write('Skipping Windows code signing for non-release build.\n');
} else {
  try {
    const config = windowsSigningConfig();
    if (config.mode === 'pfx') signWithPfx(target, config.timestampUrl);
    else signWithAzure(target);
    process.stdout.write(`Signed ${path.basename(target)} using ${config.mode}.\n`);
  } catch (error) {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  }
}

function signWithPfx(target, timestampUrl) {
  const certificate = path.join(os.tmpdir(), `auralis-signing-${process.pid}.pfx`);
  try {
    fs.writeFileSync(certificate, Buffer.from(process.env.WINDOWS_CERTIFICATE, 'base64'), {
      mode: 0o600,
    });
    run('signtool.exe', [
      'sign',
      '/fd',
      'SHA256',
      '/td',
      'SHA256',
      '/tr',
      timestampUrl,
      '/f',
      certificate,
      '/p',
      process.env.WINDOWS_CERTIFICATE_PASSWORD,
      target,
    ]);
  } finally {
    fs.rmSync(certificate, { force: true });
  }
}

function signWithAzure(target) {
  run('artifact-signing-cli', [
    '-e',
    process.env.AZURE_ARTIFACT_SIGNING_ENDPOINT,
    '-a',
    process.env.AZURE_ARTIFACT_SIGNING_ACCOUNT,
    '-c',
    process.env.AZURE_ARTIFACT_SIGNING_PROFILE,
    '-d',
    'Auralis',
    target,
  ]);
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
    throw new Error(`Windows signing failed: ${detail}`);
  }
}
