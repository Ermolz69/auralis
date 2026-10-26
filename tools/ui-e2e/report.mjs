import fsp from 'node:fs/promises';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { createRequire } from 'node:module';
import {
  Blocked,
  UnconfirmedCleanup,
  json,
  overall,
  owned,
  runs,
  idPattern,
  root,
} from './core.mjs';

const escape = (value) =>
  String(value ?? '').replace(
    /[&<>"']/g,
    (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c],
  );
export class Report {
  constructor(run, layer, planned) {
    this.run = run;
    this.data = {
      driver: {
        node: process.version,
        playwright: createRequire(path.join(root, 'apps/desktop/package.json'))(
          'playwright/package.json',
        ).version,
      },
      version: 1,
      run_id: run.id,
      layer,
      started_at: new Date().toISOString(),
      steps: planned.map((id) => ({ id, status: 'not_run' })),
      artifacts: [],
      versions: {},
    };
  }
  async step(id, work, page) {
    const step = this.data.steps.find((item) => item.id === id);
    if (!step || step.status !== 'not_run') throw new Error(`Unexpected or repeated step ${id}`);
    const start = Date.now();
    try {
      const evidence = await work();
      step.status = 'passed';
      if (evidence !== undefined) step.evidence = evidence;
    } catch (error) {
      if (error instanceof UnconfirmedCleanup) this.data.cleanup_unconfirmed = true;
      if (this.abortReason && id !== 'cleanup') error = this.abortReason;
      step.status = error instanceof Blocked ? 'blocked' : 'failed';
      step.error = String(error.message).slice(0, 8000);
    }
    step.duration_ms = Date.now() - start;
    if (page && !page.isClosed()) {
      const image = `${id.replace(/[^a-z0-9-]/gi, '-')}.png`;
      try {
        await page.screenshot({ path: path.join(this.run.directory, image), timeout: 5000 });
        step.screenshot = image;
      } catch (error) {
        step.screenshot_error = String(error.message).slice(0, 500);
        if (step.status === 'passed') {
          step.status = 'failed';
          step.error = `Required screenshot was not retained: ${step.screenshot_error}`;
        }
      }
    }
    await this.save();
    process.stdout.write(`${step.status.toUpperCase()} ${id}\n`);
    return step.status === 'passed';
  }
  async save() {
    this.data.status = overall(this.data.steps);
    this.data.updated_at = new Date().toISOString();
    this.data.artifacts = (await fsp.readdir(this.run.directory)).filter((name) =>
      /\.(png|zip|log|txt|jsonl)$/.test(name),
    );
    await fsp.writeFile(
      path.join(this.run.directory, 'report.json'),
      JSON.stringify(this.data, null, 2),
    );
    const rows = this.data.steps
      .map(
        (step) =>
          `<tr><td>${escape(step.id)}</td><td>${escape(step.status)}</td><td>${step.duration_ms ?? ''}</td><td><pre>${escape(step.error ?? JSON.stringify(step.evidence ?? '', null, 2))}</pre>${step.screenshot ? `<a href="${escape(step.screenshot)}">Screenshot</a>` : ''}</td></tr>`,
      )
      .join('');
    await fsp.writeFile(
      path.join(this.run.directory, 'index.html'),
      `<!doctype html><meta charset="utf-8"><title>Auralis UI ${escape(this.run.id)}</title><style>body{font:15px system-ui;margin:2rem;background:#15171d;color:#eee}table{border-collapse:collapse;width:100%}td,th{border:1px solid #555;padding:.6rem;text-align:left}pre{white-space:pre-wrap;overflow-wrap:anywhere}a{color:#8acfff}</style><h1>${escape(this.data.layer)} · ${escape(this.data.status)}</h1><p>${escape(this.run.id)}</p><a href="report.json">JSON report</a> · <a href="junit.xml">JUnit report</a><p>${this.data.artifacts.map((name) => `<a href="${escape(name)}">${escape(name)}</a>`).join(' · ')}</p><table><tr><th>Step</th><th>Status</th><th>ms</th><th>Evidence</th></tr>${rows}</table>`,
    );
    const cases = this.data.steps
      .map(
        (step) =>
          `<testcase name="${escape(step.id)}" time="${(step.duration_ms ?? 0) / 1000}">${step.status === 'passed' ? '' : step.status === 'failed' ? `<failure message="${escape(step.error)}"/>` : `<skipped message="${escape(step.error ?? step.status)}"/>`}</testcase>`,
      )
      .join('');
    await fsp.writeFile(
      path.join(this.run.directory, 'junit.xml'),
      `<?xml version="1.0" encoding="UTF-8"?><testsuite name="${escape(this.data.layer)}" tests="${this.data.steps.length}" failures="${this.data.steps.filter((s) => s.status === 'failed').length}" skipped="${this.data.steps.filter((s) => ['blocked', 'not_run'].includes(s.status)).length}">${cases}</testsuite>`,
    );
  }
  exitCode() {
    return this.data.status === 'passed' ? 0 : this.data.status === 'failed' ? 1 : 2;
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const id = process.argv[2];
  if (!idPattern.test(id ?? '')) throw new Error('RUN must be an existing run ID');
  const directory = owned(runs, path.join(runs, id));
  const report = await json(path.join(directory, 'report.json'));
  process.stdout.write(
    `${report.layer}: ${report.status}\n${report.steps.map((s) => `${s.status}: ${s.id}${s.error ? ` — ${s.error}` : ''}`).join('\n')}\n${path.join(directory, 'index.html')}\n`,
  );
  if (report.layer.startsWith('desktop-')) {
    const { profileFacts } = await import('./readiness.mjs');
    const facts = await profileFacts(path.join(directory, 'profile')).catch((error) => ({
      unavailable: String(error.message),
    }));
    process.stdout.write(`Current isolated profile: ${JSON.stringify(facts)}\n`);
  }
}
