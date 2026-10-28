import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { setTimeout as delay } from 'node:timers/promises';
import { Blocked, UnconfirmedCleanup } from './core.mjs';
import { OwnedTree } from './lineage.mjs';

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
    "Get-CimInstance Win32_Process -ErrorAction Stop | Select-Object @{n='pid';e={$_.ProcessId}},@{n='parent';e={$_.ParentProcessId}},@{n='name';e={$_.Name}},@{n='rss_bytes';e={[long]$_.WorkingSetSize}},@{n='created';e={$_.CreationDate.ToUniversalTime().ToString('o')}} | ConvertTo-Json -Compress",
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
export async function stopOwned(child, tree = child?.pid ? new OwnedTree(child.pid) : null) {
  if (!child?.pid) return { confirmed: true, launched: false };
  if (!tree.root) tree.observe(await inventory());
  const exited = () => child.exitCode !== null || child.signalCode !== null;
  let forced = false;
  if (!exited()) {
    tree.observe(await inventory());
    await powershell(
      `$p=Get-Process -Id ${Number(child.pid)} -ErrorAction SilentlyContinue; if($p){[void]$p.CloseMainWindow()}`,
    ).catch(() => {});
    const deadline = Date.now() + 15000;
    while (!exited() && Date.now() < deadline) {
      tree.observe(await inventory());
      await delay(200);
    }
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
    const owned = tree.observe(active);
    if (exited() && owned.length === 0) return { confirmed: true, forced, owned_processes: tree.seen.size, remaining_processes: 0 };
    await delay(500);
  }
  throw new UnconfirmedCleanup('Owned process cleanup is unconfirmed; another UI/model attempt is prohibited');
}
