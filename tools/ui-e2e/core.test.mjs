import assert from 'node:assert/strict';
import fsp from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { test } from 'node:test';
import { owned, overall, validateSpec, Blocked } from './core.mjs';
import { Report } from './report.mjs';
import { inferenceDraft } from './spec-defaults.mjs';

test('owned paths reject roots, traversal and sibling prefix collisions', () => {
  const root = path.join(os.tmpdir(), 'ui-owned');
  assert.throws(() => owned(root, root));
  assert.throws(() => owned(root, path.join(root, '..', 'outside')));
  assert.throws(() => owned(root, `${root}-other/file`));
  assert.equal(owned(root, path.join(root, 'run', 'profile')), path.join(root, 'run', 'profile'));
});
test('blocked and unexecuted work never turn a suite green', () => {
  assert.equal(overall([]), 'not_run');
  assert.equal(overall([{ status: 'passed' }, { status: 'blocked' }]), 'blocked');
  assert.equal(overall([{ status: 'passed' }, { status: 'not_run' }]), 'not_run');
  assert.equal(overall([{ status: 'failed' }, { status: 'blocked' }]), 'failed');
});
test('finite real specifications require a pinned candidate and engine order', () => {
  const spec = {
    version: 1,
    case_id: 'contract-test',
    candidate_sha256: 'a'.repeat(64),
    source_subtitle_sha256: 'b'.repeat(64),
    source_url: 'https://www.youtube.com/watch?v=HRaa8kb1EhE',
    track: { language: 'zh', format: 'srt', automatic: false },
    max_wall_seconds: 600,
    engines: ['voxcpm2', 'espeech'].map((engine) => ({
      engine_id: engine,
      profile_id: 'profile',
      voice_id: 'voice',
      reference_id: 'reference',
    })),
  };
  assert.equal(validateSpec(spec), spec);
  assert.throws(() => validateSpec({ ...spec, engines: [...spec.engines].reverse() }));
  assert.throws(() => validateSpec({ ...spec, max_wall_seconds: Infinity }));
  assert.throws(() => validateSpec({ ...spec, candidate_sha256: 'latest' }));
  const full = {
    ...spec,
    ...inferenceDraft(),
    allow_inference: true,
    export_video: true,
    runtime_seed: { files: [{ path: 'models/model.gguf' }] },
  };
  full.candidate_receipt_sha256 = 'a'.repeat(64);
  full.engine_identities = full.engines.map((e) => ({
    ...e,
    profile_sha256: 'c'.repeat(64),
    voice_manifest_sha256: 'd'.repeat(64),
    reference_audio_sha256: 'e'.repeat(64),
    model_revision: 'f'.repeat(40),
  }));
  assert.equal(validateSpec(full), full);
  for (const change of [
    { expected_cue_count: 4003 },
    { case_scope: 'short-full' },
    { export_video: false },
    { exercise_cancellation: true },
    { resource_budget: { ...full.resource_budget, max_vram_bytes: 16 * 1024 ** 3 } },
  ])
    assert.throws(() => validateSpec({ ...full, ...change }));
  const diagnostic = { ...full, ...inferenceDraft('diagnostic'), allow_inference: true };
  assert.equal(validateSpec(diagnostic), diagnostic);
  assert.throws(() => validateSpec({ ...diagnostic, diagnostic_text: '' }));
});
test('reports retain blockers, unexecuted dependencies, escaped HTML and nonzero exit', async () => {
  const directory = await fsp.mkdtemp(path.join(os.tmpdir(), 'auralis-ui-report-'));
  try {
    const report = new Report({ id: 'report-test', directory }, 'desktop-real', [
      'preflight',
      'render',
    ]);
    assert.equal(
      await report.step('preflight', () => {
        throw new Blocked('<missing model>');
      }),
      false,
    );
    assert.equal(report.exitCode(), 2);
    assert.equal(report.data.steps[1].status, 'not_run');
    assert.match(
      await fsp.readFile(path.join(directory, 'index.html'), 'utf8'),
      /&lt;missing model&gt;/,
    );
    await assert.rejects(
      report.step('preflight', () => {}),
      /repeated/,
    );
  } finally {
    await fsp.rm(owned(os.tmpdir(), directory), { recursive: true, force: true });
  }
});
