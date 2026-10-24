import assert from 'node:assert/strict';
import fsp from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { test } from 'node:test';
import { hash, owned, UnconfirmedCleanup } from './core.mjs';
import { Report } from './report.mjs';
import { candidateIdentity } from './session.mjs';
import { inspectWav } from './wav.mjs';
import { verifySources } from './media.mjs';
import { DatabaseSync } from 'node:sqlite';
import { validateSeedPath } from './seed.mjs';

test('component seeds cannot carry projects, application jobs or browser state into startup', () => {
  for (const name of [
    'auralis.sqlite',
    'auralis-translate.sqlite',
    'projects/one/file',
    'webview2/main/file',
    'assets/../auralis.sqlite',
    'tts/../../auralis.sqlite',
  ])
    assert.throws(() => validateSeedPath(name));
  for (const name of [
    'tts-runtime.json',
    'tts/runtime/base/python.exe',
    'models/model.gguf',
    'translation-packages/cpu/runtime.exe',
  ])
    assert.doesNotThrow(() => validateSeedPath(name));
});

test('source evidence resolves managed keys below projects and rejects changed subtitles', async () => {
  const profile = await fsp.mkdtemp(path.join(os.tmpdir(), 'auralis-ui-source-'));
  try {
    const managed = path.join(profile, 'projects', 'one');
    await fsp.mkdir(managed, { recursive: true });
    await fsp.writeFile(
      path.join(managed, 'source.srt'),
      '1\n00:00:00,000 --> 00:00:01,000\n你好。\n',
    );
    await fsp.writeFile(path.join(managed, 'video.mp4'), 'source identity fixture');
    const sourceHash = await hash(path.join(managed, 'source.srt'));
    const db = new DatabaseSync(path.join(profile, 'auralis.sqlite'));
    db.exec(
      'CREATE TABLE artifacts(id TEXT,kind TEXT,location_kind TEXT,location_value TEXT,state TEXT)',
    );
    for (const [id, kind, file] of [
      ['srt', 'original_subtitle', 'source.srt'],
      ['video', 'downloaded_video', 'video.mp4'],
    ])
      db.prepare('INSERT INTO artifacts VALUES (?,?,?,?,?)').run(
        id,
        kind,
        'StorageKey',
        `one/${file}`,
        'ready',
      );
    db.close();
    assert.equal(
      (await verifySources(profile, { source_subtitle_sha256: sourceHash })).files.length,
      2,
    );
    await fsp.writeFile(path.join(managed, 'source.srt'), 'changed');
    await assert.rejects(
      verifySources(profile, { source_subtitle_sha256: sourceHash }),
      /source changed/,
    );
  } finally {
    await fsp.rm(owned(os.tmpdir(), profile), { recursive: true, force: true });
  }
});

test('candidate verification rejects changed executables and escaped resources before launch', async () => {
  const base = await fsp.mkdtemp(path.join(os.tmpdir(), 'auralis-ui-candidate-'));
  try {
    const bytes = Buffer.from('identity fixture');
    const temporary = path.join(base, 'source');
    await fsp.writeFile(temporary, bytes);
    const sha256 = await hash(temporary);
    const directory = path.join(base, 'candidates', sha256);
    await fsp.mkdir(path.join(directory, 'binaries'), { recursive: true });
    const exe = path.join(directory, 'auralis-app.exe');
    await fsp.writeFile(exe, bytes);
    const resources = [];
    for (const tool of ['ffmpeg', 'ffprobe', 'yt-dlp']) {
      const name = `binaries/${tool}-x86_64-pc-windows-msvc.exe`;
      await fsp.writeFile(path.join(directory, name), bytes);
      resources.push({ path: name, sha256 });
    }
    await fsp.writeFile(path.join(base, 'candidate.json'), JSON.stringify({ sha256 }));
    const receipt = path.join(directory, 'candidate.json');
    await fsp.writeFile(receipt, JSON.stringify({ version: 2, sha256, resources }));
    assert.equal((await candidateIdentity(base)).receipt.sha256, sha256);
    await fsp.writeFile(exe, 'changed');
    await assert.rejects(candidateIdentity(base), /changed/);
    await fsp.writeFile(exe, bytes);
    await fsp.writeFile(
      receipt,
      JSON.stringify({
        version: 2,
        sha256,
        resources: [...resources, { path: '../escape', sha256 }],
      }),
    );
    await assert.rejects(candidateIdentity(base), /owned test directory/);
  } finally {
    await fsp.rm(owned(os.tmpdir(), base), { recursive: true, force: true });
  }
});
test('unconfirmed cleanup remains explicit even when later bookkeeping succeeds', async () => {
  const directory = await fsp.mkdtemp(path.join(os.tmpdir(), 'auralis-ui-uncertain-'));
  try {
    const report = new Report({ id: 'uncertain', directory }, 'desktop-real', [
      'release',
      'cleanup',
    ]);
    await report.step('release', () => {
      throw new UnconfirmedCleanup('Unconfirmed admission release');
    });
    await report.step('cleanup', () => ({ confirmed: true }));
    assert.equal(report.data.cleanup_unconfirmed, true);
    assert.equal(report.exitCode(), 1);
  } finally {
    await fsp.rm(owned(os.tmpdir(), directory), { recursive: true, force: true });
  }
});
test('WAV measurements use actual PCM, reject truncation and count full-scale samples', async () => {
  const directory = await fsp.mkdtemp(path.join(os.tmpdir(), 'auralis-ui-wav-'));
  const file = path.join(directory, 'fixture.wav');
  try {
    const bytes = Buffer.alloc(48);
    bytes.write('RIFF');
    bytes.writeUInt32LE(40, 4);
    bytes.write('WAVEfmt ', 8);
    bytes.writeUInt32LE(16, 16);
    bytes.writeUInt16LE(1, 20);
    bytes.writeUInt16LE(1, 22);
    bytes.writeUInt32LE(48000, 24);
    bytes.writeUInt32LE(96000, 28);
    bytes.writeUInt16LE(2, 32);
    bytes.writeUInt16LE(16, 34);
    bytes.write('data', 36);
    bytes.writeUInt32LE(4, 40);
    bytes.writeInt16LE(-32768, 44);
    bytes.writeInt16LE(120, 46);
    await fsp.writeFile(file, bytes);
    const measured = await inspectWav(file);
    assert.equal(measured.frames, 2);
    assert.equal(measured.peak_sample_abs, 32768);
    assert.equal(measured.full_scale_samples, 1);
    await fsp.writeFile(file, bytes.subarray(0, 46));
    await assert.rejects(inspectWav(file));
  } finally {
    await fsp.rm(owned(os.tmpdir(), directory), { recursive: true, force: true });
  }
});
