import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
import { loadManifest, releaseTargets, targetAssets, validateManifest } from './manifest.mjs';
import { renderNotices, renderProvenance } from './provenance.mjs';
import { verifyRepositoryPolicy } from './repository-policy.mjs';
import { prepareStagingDirectory } from './staged-files.mjs';

const rootDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const manifest = loadManifest(path.join(rootDir, 'tools/media-tools/manifest.json'));

test('pins complete media tools for every release target', () => {
  for (const target of releaseTargets) {
    const assets = targetAssets(manifest, target);
    assert.deepEqual(
      assets.map((asset) => asset.name),
      ['ffmpeg', 'ffprobe', 'yt-dlp'],
    );
    assert.equal(new Set(assets.map((asset) => asset.output)).size, 3);
  }
});

test('rejects untrusted downloads, unsafe outputs and missing target tools', () => {
  const invalid = structuredClone(manifest);
  invalid.targets['x86_64-pc-windows-msvc'].ffmpeg.url = 'http://example.com/ffmpeg.exe';
  invalid.targets['x86_64-pc-windows-msvc'].ffprobe.output = '../ffprobe.exe';
  delete invalid.targets['x86_64-unknown-linux-gnu']['yt-dlp'];
  assert.throws(
    () => validateManifest(invalid),
    /trusted HTTPS download host[\s\S]*unsafe output filename[\s\S]*missing yt-dlp/,
  );
});

test('rejects a media tool without a declared license', () => {
  const invalid = structuredClone(manifest);
  invalid.tools.ffmpeg.license = 'missing-license';

  assert.throws(() => validateManifest(invalid), /must reference a declared license/);
});

test('renders deterministic provenance without timestamps or local paths', () => {
  const target = 'x86_64-pc-windows-msvc';
  const provenance = renderProvenance(manifest, target);
  assert.equal(provenance, renderProvenance(manifest, target));
  assert.match(provenance, /2026\.08\.19/);
  assert.doesNotMatch(provenance, /downloadedAt|[A-Z]:\\/);
  assert.match(renderNotices(manifest, target), /GPL-3\.0-or-later/);
});

test('repository build and release paths enforce media-tool preparation and bundle verification', () => {
  assert.doesNotThrow(() => verifyRepositoryPolicy(rootDir));
  assert.equal(fs.existsSync(path.join(rootDir, 'src-tauri/binaries/.gitkeep')), false);
});

test('staging removes known files from another target and rejects unknown entries', async (context) => {
  const binariesDir = fs.mkdtempSync(path.join(os.tmpdir(), 'auralis-media-tools-test-'));
  context.after(() => fs.rmSync(binariesDir, { recursive: true, force: true }));
  const stale = path.join(binariesDir, 'ffmpeg-obsolete-target');
  fs.writeFileSync(stale, 'stale');

  await prepareStagingDirectory({
    manifest,
    target: 'x86_64-pc-windows-msvc',
    binariesDir,
  });

  assert.equal(fs.existsSync(stale), false);
  fs.writeFileSync(path.join(binariesDir, 'unexpected.exe'), 'unknown');
  await assert.rejects(
    prepareStagingDirectory({
      manifest,
      target: 'x86_64-pc-windows-msvc',
      binariesDir,
    }),
    /Unexpected entry in media-tools staging directory/,
  );
});
