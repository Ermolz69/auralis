import { spawn } from 'node:child_process';
import path from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { root, Blocked, UnconfirmedCleanup } from './core.mjs';

export async function acquireDownloadAdmission() {
  const guardian = spawn(
    'powershell.exe',
    [
      '-NoProfile',
      '-NonInteractive',
      '-ExecutionPolicy',
      'Bypass',
      '-File',
      path.join(root, 'crates/common/src/model_admission.ps1'),
      '-ModelFree',
    ],
    { windowsHide: true, stdio: ['pipe', 'pipe', 'pipe'] },
  );
  let output = '',
    spawnError;
  guardian.stdin.on('error', (error) => {
    spawnError = error;
  });
  guardian.stdout.on('data', (block) => {
    output = (output + block.toString()).slice(-8000);
  });
  guardian.stderr.on('data', () => {});
  guardian.on('error', (error) => {
    spawnError = error;
  });
  const deadline = Date.now() + 30000;
  while (
    !output.includes('\n') &&
    !spawnError &&
    guardian.exitCode === null &&
    Date.now() < deadline
  )
    await delay(100);
  if (spawnError) throw spawnError;
  if (output.split(/\r?\n/)[0] !== 'READY') {
    guardian.stdin.end();
    const stop = Date.now() + 30000;
    while (guardian.exitCode === null && Date.now() < stop) await delay(100);
    if (guardian.exitCode === null)
      throw new UnconfirmedCleanup('Download admission acquisition outcome is unconfirmed');
    throw new Blocked(`Shared download admission refused: ${output.trim() || 'response timeout'}`);
  }
  return async () => {
    guardian.stdin.end('release\n');
    const stop = Date.now() + 30000;
    while (guardian.exitCode === null && Date.now() < stop) await delay(100);
    if (guardian.exitCode !== 0)
      throw new UnconfirmedCleanup(
        'Download admission release is unconfirmed; do not start another component',
      );
    return { guardian_pid: guardian.pid, released: true };
  };
}
