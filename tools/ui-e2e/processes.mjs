import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { setTimeout as delay } from 'node:timers/promises';
import { Blocked, UnconfirmedCleanup } from './core.mjs';
import { OwnedTree } from './lineage.mjs';

const exec = promisify(execFile);
export async function powershell(code, timeout = 15000) {
  const { stdout } = await exec(
    'powershell.exe',
    ['-NoProfile', '-NonInteractive', '-Command', code],
    { windowsHide: true, timeout, maxBuffer: 4 * 1024 * 1024 },
  );
  return stdout.trim();
}
export async function inventory(timeout = 15000) {
  const data = await powershell(
    "Get-CimInstance Win32_Process -ErrorAction Stop | Select-Object @{n='pid';e={$_.ProcessId}},@{n='parent';e={$_.ParentProcessId}},@{n='name';e={$_.Name}},@{n='rss_bytes';e={[long]$_.WorkingSetSize}},@{n='created';e={$_.CreationDate.ToUniversalTime().ToString('o')}} | ConvertTo-Json -Compress",
    timeout,
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
async function signalKnown(records, close, timeout = 15000) {
  const payload = Buffer.from(
    JSON.stringify(records.map((p) => ({ pid: p.pid, created: p.created }))),
  ).toString('base64');
  return powershell(
    `$items=ConvertFrom-Json ([Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('${payload}'))); foreach($item in $items){$p=Get-Process -Id ([int]$item.pid) -ErrorAction SilentlyContinue; if($p){$expected=[DateTime]::Parse($item.created).ToUniversalTime().ToString('yyyyMMddHHmmssffffff'); try {if($p.StartTime.ToUniversalTime().ToString('yyyyMMddHHmmssffffff') -eq $expected){${close ? '[void]$p.CloseMainWindow()' : '$p.Kill()'}}} catch [InvalidOperationException] {}}}`,
    timeout,
  );
}
export async function stopOwned(child, tree = child?.pid ? new OwnedTree(child.pid) : null) {
  if (!child?.pid) return { confirmed: true, launched: false };
  const cleanupDeadline = Date.now() + 60000;
  const left = () => {
    const ms = Math.min(15000, cleanupDeadline - Date.now());
    if (ms <= 0) throw new UnconfirmedCleanup('Owned cleanup deadline exhausted');
    return ms;
  };
  if (!tree.root) {
    try {
      tree.observe(await inventory(left()));
    } catch (error) {
      throw new UnconfirmedCleanup(error.message);
    }
  }
  const exited = () => child.exitCode !== null || child.signalCode !== null;
  let forced = false;
  if (!exited()) {
    tree.observe(await inventory(left()));
    await signalKnown([tree.seen.get(tree.root)], true, left()).catch(() => {});
    const deadline = Math.min(cleanupDeadline, Date.now() + 30000);
    while (!exited() && Date.now() < deadline) {
      tree.observe(await inventory(left()));
      await delay(200);
    }
    if (!exited()) {
      forced = true;
      await signalKnown(tree.observe(await inventory(left())).reverse(), false, left()).catch(
        () => {},
      );
    }
  }
  const deadline = Math.min(cleanupDeadline, Date.now() + 15000);
  while (Date.now() < deadline) {
    const active = await inventory(left());
    const owned = tree.observe(active);
    try {
      tree.assertKnown();
    } catch (error) {
      throw new UnconfirmedCleanup(error.message);
    }
    if (exited() && owned.length === 0)
      return { confirmed: true, forced, owned_processes: tree.seen.size, remaining_processes: 0 };
    await delay(500);
  }
  throw new UnconfirmedCleanup(
    'Owned process cleanup is unconfirmed; another UI/model attempt is prohibited',
  );
}
