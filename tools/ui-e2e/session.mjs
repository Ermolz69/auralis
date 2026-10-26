import fsp from 'node:fs/promises';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { createRequire } from 'node:module';
import { setTimeout as delay } from 'node:timers/promises';
import net from 'node:net';
import { root, storage, hash, json, owned, UnconfirmedCleanup, Blocked } from './core.mjs';
import { stopOwned, inventory } from './processes.mjs';
import { OwnedTree } from './lineage.mjs';
import { profileFacts } from './readiness.mjs';

export async function candidateIdentity(base = storage, remaining = () => Infinity) {
  const pointer = await json(path.join(base, 'candidate.json')).catch((error) => {
    if (error.code === 'ENOENT')
      throw new Blocked('Retained ordinary release candidate is unavailable; run task ui:build');
    throw error;
  });
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
  if (![2, 3].includes(receipt.version))
    throw new Error('Rebuild the candidate with source identity verification');
  if (receipt.version === 3) {
    if (receipt.mode !== 'ordinary-release') throw new Error('Expected an ordinary release build');
    if (
      !Array.isArray(receipt.build_inputs?.sources) ||
      !Array.isArray(receipt.build_inputs?.dependency_sources) ||
      !receipt.build_inputs.dependency_sources.length
    )
      throw new Error('Complete build input manifest is missing');
    for (const required of [
      'package.json',
      'pnpm-lock.yaml',
      'src-tauri/build.rs',
      'src-tauri/tauri.conf.json',
      'apps/desktop/index.html',
    ])
      if (!receipt.build_inputs.sources.some((item) => item.path === required))
        throw new Error(`Missing build input ${required}`);
  }
  const executable = path.join(directory, 'auralis-app.exe');
  if (receipt.sha256 !== pointer.sha256 || (await hash(executable, remaining)) !== receipt.sha256)
    throw new Error('Candidate executable changed');
  for (const resource of receipt.resources) {
    const file = owned(directory, path.join(directory, resource.path));
    if ((await hash(file, remaining)) !== resource.sha256)
      throw new Error(`Candidate resource changed: ${resource.path}`);
  }
  return {
    directory,
    executable,
    receipt,
    receipt_sha256: await hash(path.join(directory, 'candidate.json'), remaining),
  };
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
  const handle = await fsp.open(file, 'wx').catch((error) => {
    if (error.code === 'EEXIST')
      throw new Blocked(
        'An existing UI session lock prevents another attempt; inspect its owning process and cleanup report',
      );
    throw error;
  });
  await handle.writeFile(
    JSON.stringify({ pid: process.pid, started_at: new Date().toISOString() }),
  );
  return async (confirmed) => {
    await handle.close();
    if (confirmed) await fsp.unlink(file);
  };
}
export class DesktopSession {
  constructor(run, candidate, policy = {}) {
    this.run = run;
    this.candidate = candidate;
    this.policy = policy;
    this.log = '';
    this.errors = [];
  }
  async start(budget = () => 120000) {
    const { chromium } = createRequire(path.join(root, 'apps/desktop/package.json'))('playwright');
    const deadline = Date.now() + Math.min(120000, budget());
    this.launchNumber = (this.launchNumber ?? 0) + 1;
    this.spawnError = null;
    this.ownershipError = null;
    this.closed = null;
    this.log = '';
    const profile = path.join(this.run.directory, 'profile');
    await fsp.mkdir(profile, { recursive: true });
    if (this.policy.tts_execution_budget_seconds !== undefined) {
      const config = await json(path.join(profile, 'tts-runtime.json'));
      if (config.execution_budget_seconds !== this.policy.tts_execution_budget_seconds)
        throw new Error('Frozen host TTS execution budget changed before launch');
    }
    if (
      this.policy.tts_runtime_sha256 &&
      (await hash(path.join(profile, 'tts-runtime.json'), budget)) !==
        this.policy.tts_runtime_sha256
    )
      throw new Error('Applied isolated TTS configuration bytes changed before launch');
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
    const baseline = await inventory();
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
    this.tree = new OwnedTree(this.child.pid, baseline);
    await this.processes();
    this.ownershipTimer = setInterval(() => {
      if (!this.ownershipRead)
        this.ownershipRead = this.processes()
          .catch((error) => {
            this.ownershipError = error;
            void this.page?.close().catch(() => {});
          })
          .finally(() => {
            this.ownershipRead = null;
          });
    }, 1000);
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
    await this.page.context().tracing.start({ screenshots: true, snapshots: true, sources: false });
    this.tracing = true;
    await this.page.getByRole('button', { name: 'Settings', exact: true }).waitFor();
    await this.page.screenshot({
      path: path.join(this.run.directory, `launch-${this.launchNumber}.png`),
      timeout: 5000,
    });
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
  async processes() {
    const active = this.tree.observe(await inventory());
    this.tree.assertKnown();
    return active;
  }
  async close() {
    if (this.closed) return this.closed;
    clearInterval(this.ownershipTimer);
    await this.ownershipRead;
    let traceError;
    if (this.page && this.tracing) {
      let timer;
      await Promise.race([
        this.page
          .context()
          .tracing.stop({ path: path.join(this.run.directory, `trace-${this.launchNumber}.zip`) }),
        new Promise((_, reject) => {
          timer = setTimeout(
            () => reject(new Error('Trace finalization exceeded 10 seconds')),
            10000,
          );
        }),
      ])
        .catch((error) => {
          traceError = error;
        })
        .finally(() => clearTimeout(timer));
    }
    this.tracing = false;
    let released;
    try {
      released = await stopOwned(this.child, this.tree);
    } finally {
      if (this.browser) await this.browser.close().catch(() => {});
      await fsp.writeFile(
        path.join(this.run.directory, `processes-${this.launchNumber}.json`),
        JSON.stringify(
          {
            seen: [...(this.tree?.seen?.values() ?? [])],
            ambiguous: [...(this.tree?.ambiguous?.values() ?? [])],
            release: released ?? null,
            observer_error: this.ownershipError?.message ?? null,
          },
          null,
          2,
        ),
      );
      await fsp.writeFile(
        path.join(this.run.directory, `desktop-${this.launchNumber}.log`),
        this.log,
      );
    }
    this.closed = released;
    if (this.ownershipError)
      throw new UnconfirmedCleanup(`Process observer failed: ${this.ownershipError.message}`);
    if (traceError) throw new Error(`Diagnostic trace was not retained: ${traceError.message}`);
    return released;
  }
}
