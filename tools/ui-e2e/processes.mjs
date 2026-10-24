import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { setTimeout as delay } from 'node:timers/promises';
import { Blocked } from './core.mjs';

const exec = promisify(execFile);
export async function powershell(code) {
  const { stdout } = await exec(
    'powershell.exe',
    ['-NoProfile', '-NonInteractive', '-Command', code],
    { windowsHide: true, timeout: 15000, maxBuffer: 4 * 1024 * 1024 },
  );
  return stdout.trim();
}
export async function inventory() {
  const data = await powershell(
    "Get-CimInstance Win32_Process | Select-Object @{n='pid';e={$_.ProcessId}},@{n='parent';e={$_.ParentProcessId}},@{n='name';e={$_.Name}},@{n='created';e={$_.CreationDate.ToUniversalTime().ToString('o')}} | ConvertTo-Json -Compress",
  );
  return data ? [].concat(JSON.parse(data)) : [];
}
const model = /^(pythonw?|llama[^.]*|ffmpeg[^.]*|ffprobe[^.]*|yt-dlp[^.]*|whisper[^.]*)\.exe$/i;
export async function requireQuiescent() {
  const active = (await inventory()).filter((p) => model.test(p.name));
  if (active.length)
    throw new Blocked(
      `Competing component processes: ${active.map((p) => `${p.name}:${p.pid}`).join(', ')}`,
    );
  return { competing_processes: 0, observed_at: new Date().toISOString() };
}
export async function stopOwned(child) {
  if (!child?.pid) return { confirmed: true, launched: false };
  const exited = () => child.exitCode !== null || child.signalCode !== null;
  let forced = false;
  if (!exited()) {
    await powershell(
      `$p=Get-Process -Id ${Number(child.pid)} -ErrorAction SilentlyContinue; if($p){[void]$p.CloseMainWindow()}`,
    ).catch(() => {});
    const deadline = Date.now() + 15000;
    while (!exited() && Date.now() < deadline) await delay(200);
    if (!exited()) {
      forced = true;
      await exec('taskkill.exe', ['/PID', String(child.pid), '/T', '/F'], {
        windowsHide: true,
        timeout: 15000,
      }).catch(() => {});
    }
  }
  const deadline = Date.now() + 15000;
  while (Date.now() < deadline) {
    const active = await inventory();
    if (exited() && !active.some((p) => model.test(p.name))) return { confirmed: true, forced };
    await delay(500);
  }
  throw new Error('Process cleanup is unconfirmed; another UI/model attempt is prohibited');
}
