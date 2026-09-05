import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import {
  buildUpdaterManifest,
  collectReleaseAssets,
  prepareReleaseAssets,
} from './release-assets.mjs';

const packageFiles = [
  ['windows/Auralis.msi', 'msi'],
  ['windows/Auralis.msi.sig', 'msi-signature'],
  ['windows/Auralis-setup.exe', 'nsis'],
  ['windows/Auralis-setup.exe.sig', 'nsis-signature'],
  ['macos/Auralis.dmg', 'dmg'],
  ['macos/Auralis.app.tar.gz', 'mac-updater'],
  ['macos/Auralis.app.tar.gz.sig', 'mac-signature'],
  ['linux/Auralis.AppImage', 'appimage'],
  ['linux/Auralis.AppImage.sig', 'linux-signature'],
  ['linux/auralis.deb', 'deb'],
  ['linux/auralis.rpm', 'rpm'],
];
const metadata = {
  version: '0.2.0',
  tag: 'app-v0.2.0',
  repository: 'Ermolz69/auralis',
  notes: 'Verified release',
};

test('validates packages and updater signatures for every release platform', () => {
  withPackages((root) => {
    assert.equal(collectReleaseAssets(root, 'windows').length, 4);
    assert.equal(collectReleaseAssets(root, 'macos').length, 3);
    assert.equal(collectReleaseAssets(root, 'linux').length, 4);
  });
});

test('builds the static GitHub updater manifest from signed bundles', () => {
  withPackages((root) => {
    const manifest = buildUpdaterManifest(collectReleaseAssets(root, 'all'), metadata);
    assert.equal(manifest.version, '0.2.0');
    assert.equal(manifest.platforms['linux-x86_64'].signature, 'linux-signature');
    assert.equal(manifest.platforms['windows-x86_64'].signature, 'nsis-signature');
    assert.equal(manifest.platforms['darwin-aarch64'].signature, 'mac-signature');
    assert.equal(
      manifest.platforms['windows-x86_64'].url,
      'https://github.com/Ermolz69/auralis/releases/download/app-v0.2.0/Auralis-setup.exe',
    );
  });
});

test('prepares updater metadata and deterministic SHA-256 checksums', () => {
  withPackages((root) => {
    const output = path.join(path.dirname(root), 'publish');
    const assets = prepareReleaseAssets(root, output, metadata);
    assert.equal(assets.length, 12);
    const published = fs.readdirSync(output).sort();
    assert.deepEqual(
      published,
      [
        ...packageFiles.map(([file]) => path.basename(file)),
        'latest.json',
        'SHA256SUMS.txt',
      ].sort(),
    );
    const expectedChecksums = packageFiles.map(
      ([file, contents]) => `${sha256(contents)}  ${path.basename(file)}`,
    );
    expectedChecksums.push(
      `${sha256(fs.readFileSync(path.join(output, 'latest.json')))}  latest.json`,
    );
    assert.deepEqual(
      fs.readFileSync(path.join(output, 'SHA256SUMS.txt'), 'utf8').trim().split('\n').sort(),
      expectedChecksums.sort(),
    );
  });
});

test('rejects incomplete assets, empty signatures and mismatched metadata', () => {
  withPackages((root) => {
    fs.rmSync(path.join(root, 'linux/Auralis.AppImage.sig'));
    assert.throws(
      () => collectReleaseAssets(root, 'all'),
      /exactly one Linux updater signature, found 0/,
    );
  });
  withPackages((root) => {
    fs.writeFileSync(path.join(root, 'windows/Auralis-setup.exe.sig'), '');
    assert.throws(
      () => buildUpdaterManifest(collectReleaseAssets(root, 'all'), metadata),
      /updater signature.*is empty/i,
    );
  });
  withPackages((root) => {
    assert.throws(
      () =>
        buildUpdaterManifest(collectReleaseAssets(root, 'all'), {
          ...metadata,
          tag: 'app-v0.3.0',
        }),
      /must equal app-v0.2.0/,
    );
  });
});

function withPackages(assertions) {
  const temporary = fs.mkdtempSync(path.join(os.tmpdir(), 'auralis-release-assets-'));
  const root = path.join(temporary, 'input');
  try {
    for (const [file, contents] of packageFiles) {
      const target = path.join(root, file);
      fs.mkdirSync(path.dirname(target), { recursive: true });
      fs.writeFileSync(target, contents);
    }
    assertions(root);
  } finally {
    fs.rmSync(temporary, { recursive: true, force: true });
  }
}

function sha256(value) {
  return crypto.createHash('sha256').update(value).digest('hex');
}
