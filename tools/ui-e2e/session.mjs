import fsp from 'node:fs/promises';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { createRequire } from 'node:module';
import { setTimeout as delay } from 'node:timers/promises';
import net from 'node:net';
import { root, storage, hash, json, owned } from './core.mjs';
import { stopOwned } from './processes.mjs';
import { profileFacts } from './readiness.mjs';

const { chromium } = createRequire(path.join(root, 'apps/desktop/package.json'))('playwright');
export async function candidateIdentity(base = storage) {
  const pointer = await json(path.join(base, 'candidate.json'));
  if (!/^[a-f0-9]{64}$/.test(pointer.sha256)) throw new Error('Invalid retained candidate');
  const directory = owned(
    path.join(base, 'candidates'),
    path.join(base, 'candidates', pointer.sha256),
  );
  const receipt = await json(path.join(directory, 'candidate.json'));
  if ((await fsp.realpath(directory)) !== directory)
    throw new Error('Candidate directory must not be redirected');
  if (
    !Array.isArray(receipt.resources) ||
    receipt.resources.length < 3 ||
    receipt.resources.length > 64
  )
    throw new Error('Candidate media resources are missing');
  for (const tool of ['ffmpeg', 'ffprobe', 'yt-dlp'])
    if (
      !receipt.resources.some((item) => item.path === `binaries/${tool}-x86_64-pc-windows-msvc.exe`)
    )
      throw new Error(`Missing packaged ${tool}`);
  if (receipt.version !== 2)
    throw new Error('Rebuild the candidate with source identity verification');
  const executable = path.join(directory, 'auralis-app.exe');
  if (receipt.sha256 !== pointer.sha256 || (await hash(executable)) !== receipt.sha256)
    throw new Error('Candidate executable changed');
  for (const resource of receipt.resources) {
    const file = owned(directory, path.join(directory, resource.path));
    if ((await hash(file)) !== resource.sha256)
      throw new Error(`Candidate resource changed: ${resource.path}`);
  }
  return { directory, executable, receipt };
}
async function port() {
  const server = net.createServer();
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  const value = server.address().port;
  await new Promise((resolve) => server.close(resolve));
  return value;
}
export async function lockSession() {
  await fsp.mkdir(storage, { recursive: true });
  const file = path.join(storage, 'session.lock');
  const handle = await fsp.open(file, 'wx');
  await handle.writeFile(
    JSON.stringify({ pid: process.pid, started_at: new Date().toISOString() }),
  );
  return async (confirmed) => {
    await handle.close();
    if (confirmed) await fsp.unlink(file);
  };
}
export class DesktopSession {
  constructor(run, candidate) {
    this.run = run;
    this.candidate = candidate;
    this.log = '';
    this.errors = [];
  }
  async start(budget = () => 120000) {
    const deadline = Date.now() + Math.min(120000, budget());
    this.launchNumber = (this.launchNumber ?? 0) + 1;
    const profile = path.join(this.run.directory, 'profile');
    await fsp.mkdir(profile, { recursive: true });
    const cdpPort = await port();
    const env = {
      ...process.env,
      AURALIS_OBSERVABILITY_ENABLED: 'false',
      RUST_LOG: 'warn,adapters_tts=info',
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${cdpPort} --remote-debugging-address=127.0.0.1`,
    };
    delete env.WEBVIEW2_USER_DATA_FOLDER;
    for (const key of Object.keys(env))
      if (key.startsWith('AURALIS_NATIVE_E2E') || key.startsWith('VITE_NATIVE_E2E'))
        delete env[key];
    this.child = spawn(this.candidate.executable, ['--profile-dir', profile], {
      cwd: this.candidate.directory,
      env,
      stdio: ['ignore', 'pipe', 'pipe'],
      windowsHide: true,
    });
    const append = (chunk) => {
      this.log = (this.log + chunk.toString()).slice(-1024 * 1024);
    };
    this.child.stdout.on('data', append);
    this.child.stderr.on('data', append);
    this.child.on('error', (error) => {
      this.spawnError = error;
    });
    while (Date.now() < deadline) {
      if (this.spawnError) throw this.spawnError;
      if (this.child.exitCode !== null)
        throw new Error(`Application exited: ${this.child.exitCode}; ${this.log.slice(-2000)}`);
      const ready = await fetch(`http://127.0.0.1:${cdpPort}/json/version`, {
        signal: AbortSignal.timeout(1000),
      })
        .then((r) => r.ok)
        .catch(() => false);
      if (ready) break;
      await delay(250);
    }
    this.browser = await chromium.connectOverCDP(`http://127.0.0.1:${cdpPort}`, {
      timeout: Math.max(1, Math.min(10000, deadline - Date.now())),
    });
    let pages = [];
    const targetDeadline = Math.min(deadline, Date.now() + 60000);
    while (Date.now() < targetDeadline) {
      pages = this.browser
        .contexts()
        .flatMap((context) => context.pages())
        .filter((page) => /^(https?:\/\/tauri\.localhost|tauri:\/\/localhost)/.test(page.url()));
      if (pages.length) break;
      await delay(200);
    }
    await fsp.writeFile(
      path.join(this.run.directory, 'targets.json'),
      JSON.stringify(
        this.browser
          .contexts()
          .flatMap((context) => context.pages())
          .map((page) => ({ url: page.url() })),
      ),
    );
    if (pages.length !== 1)
      throw new Error(`Expected one ordinary Auralis webview; found ${pages.length}`);
    this.page = pages[0];
    this.page.setDefaultTimeout(Math.max(1, Math.min(30000, deadline - Date.now())));
    this.page.on('pageerror', (error) => this.errors.push(error.message));
    await this.page
      .context()
      .tracing.start({ screenshots: false, snapshots: true, sources: false });
    await this.page.getByRole('button', { name: 'Settings', exact: true }).waitFor();
    await fsp.access(path.join(profile, 'auralis.sqlite'));
    await fsp.access(path.join(profile, 'webview2', 'main'));
    const facts = await profileFacts(profile);
    if (this.launchNumber === 1 && (facts.projects !== 0 || facts.jobs !== 0))
      throw new Error('The first UI launch requires an empty application database');
    return {
      candidate_sha256: this.candidate.receipt.sha256,
      profile_isolated: true,
      webview_url: this.page.url(),
      profile: facts,
    };
  }
  async close() {
    if (this.page)
      await this.page
        .context()
        .tracing.stop({ path: path.join(this.run.directory, `trace-${this.launchNumber}.zip`) })
        .catch(() => {});
    const released = await stopOwned(this.child);
    if (this.browser) await this.browser.close().catch(() => {});
    await fsp.writeFile(
      path.join(this.run.directory, `desktop-${this.launchNumber}.log`),
      this.log,
    );
    return released;
  }
}
