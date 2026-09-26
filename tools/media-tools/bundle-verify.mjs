import childProcess from 'node:child_process';
import fs from 'node:fs';
import fsp from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { sha256File } from './integrity.mjs';
import { targetAssets } from './manifest.mjs';
import { renderNotices, renderProvenance } from './provenance.mjs';
import { expectedStagedFilenames } from './staged-files.mjs';
import { assertNoTranslationPayload } from '../translation-delivery/policy.mjs';
import { verifyBundledApplication } from './bundled-application.mjs';

export async function verifyBuiltBundle({ rootDir, manifest, target }) {
  const bundleRoot = path.join(rootDir, 'target/release/bundle');
  const materialized = await materializeBundle(bundleRoot);
  const materializedRoot = materialized.root;
  try {
    const payload = await verifyMaterializedTree(materializedRoot, manifest, target);
    const application =
      process.platform === 'win32'
        ? await verifyBundledApplication(
            materializedRoot,
            path.join(rootDir, 'target/release/auralis-app.exe'),
          )
        : null;
    const artifact = materialized.artifact
      ? {
          file: path.relative(rootDir, materialized.artifact),
          bytes: (await fsp.stat(materialized.artifact)).size,
          sha256: await sha256File(materialized.artifact),
        }
      : null;
    return { createdAt: new Date().toISOString(), target, artifact, payload, application };
  } finally {
    if (materializedRoot !== bundleRoot && !materializedRoot.endsWith('.app')) {
      await removeTemporaryExtraction(materializedRoot);
    }
  }
}

export async function verifyMaterializedTree(root, manifest, target) {
  const files = walkFiles(root);
  assertNoTranslationPayload(files.map((file) => path.relative(root, file)));
  const bundledFiles = [];
  for (const asset of targetAssets(manifest, target)) {
    bundledFiles.push(await verifyUniqueFile(files, asset.output, asset.sha256));
  }
  for (const license of Object.values(manifest.licenses)) {
    bundledFiles.push(await verifyUniqueFile(files, license.output, license.sha256));
  }
  bundledFiles.push(
    await verifyUniqueText(
      files,
      'media-tools-provenance.json',
      renderProvenance(manifest, target),
    ),
  );
  bundledFiles.push(
    await verifyUniqueText(files, 'THIRD-PARTY-NOTICES.txt', renderNotices(manifest, target)),
  );
  await verifyBundledDirectory(bundledFiles, manifest, target);
  return { fileCount: files.length, files: files.map((file) => path.relative(root, file)).sort() };
}

function walkFiles(directory, files = []) {
  if (!fs.existsSync(directory)) return files;
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const candidate = path.join(directory, entry.name);
    if (entry.isDirectory()) walkFiles(candidate, files);
    else files.push(candidate);
  }
  return files;
}

async function materializeBundle(bundleRoot) {
  if (process.platform === 'darwin') {
    const app = findDirectories(bundleRoot).find((directory) => directory.endsWith('.app'));
    if (!app) throw new Error(`No macOS app bundle found under ${bundleRoot}`);
    return { root: app, artifact: null };
  }

  const temporary = await fsp.mkdtemp(path.join(os.tmpdir(), 'auralis-bundle-'));
  if (process.platform === 'win32') {
    const installers = walkFiles(bundleRoot).filter((file) => file.endsWith('.msi'));
    try {
      if (installers.length !== 1)
        throw new Error(
          `Expected exactly one MSI bundle under ${bundleRoot}, found ${installers.length}`,
        );
      runExtractor('msiexec.exe', ['/a', installers[0], '/qn', `TARGETDIR=${temporary}`]);
      return { root: temporary, artifact: installers[0] };
    } catch (error) {
      await removeTemporaryExtraction(temporary);
      throw error;
    }
  }
  if (process.platform === 'linux') {
    const deb = walkFiles(bundleRoot).find((file) => file.endsWith('.deb'));
    if (!deb) throw new Error(`No Debian bundle found under ${bundleRoot}`);
    runExtractor('dpkg-deb', ['-x', deb, temporary]);
    return { root: temporary, artifact: deb };
  }
  throw new Error(`Unsupported bundle verification host: ${process.platform}`);
}

async function removeTemporaryExtraction(directory) {
  const resolved = path.resolve(directory);
  if (
    path.dirname(resolved) !== path.resolve(os.tmpdir()) ||
    !path.basename(resolved).startsWith('auralis-bundle-')
  ) {
    throw new Error('Refusing cleanup outside the owned bundle extraction directory');
  }
  await fsp.rm(resolved, { recursive: true, force: true });
}

function findDirectories(directory, directories = []) {
  if (!fs.existsSync(directory)) return directories;
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    if (!entry.isDirectory()) continue;
    const candidate = path.join(directory, entry.name);
    directories.push(candidate);
    findDirectories(candidate, directories);
  }
  return directories;
}

function runExtractor(command, args) {
  const result = childProcess.spawnSync(command, args, {
    encoding: 'utf8',
    timeout: 120_000,
    windowsHide: true,
  });
  if (result.error || result.status !== 0) {
    const detail =
      result.error?.message ||
      result.stderr?.trim() ||
      result.stdout?.trim() ||
      `exit code ${result.status}`;
    throw new Error(`Bundle extraction failed: ${detail}`);
  }
}

async function verifyUniqueFile(files, basename, expectedSha256) {
  const matches = files.filter((file) => path.basename(file) === basename);
  if (matches.length !== 1)
    throw new Error(`Expected exactly one bundled ${basename}, found ${matches.length}`);
  const actualSha256 = await sha256File(matches[0]);
  if (actualSha256 !== expectedSha256)
    throw new Error(`Bundled ${basename} failed SHA-256 verification`);
  return matches[0];
}

async function verifyUniqueText(files, basename, expected) {
  const matches = files.filter((file) => path.basename(file) === basename);
  if (matches.length !== 1)
    throw new Error(`Expected exactly one bundled ${basename}, found ${matches.length}`);
  if ((await fsp.readFile(matches[0], 'utf8')) !== expected) {
    throw new Error(`Bundled ${basename} is stale`);
  }
  return matches[0];
}

async function verifyBundledDirectory(files, manifest, target) {
  const directories = new Set(files.map((file) => path.dirname(file)));
  if (directories.size !== 1) {
    throw new Error('Bundled media tools and their compliance files must share one directory');
  }
  const [directory] = directories;
  const entries = await fsp.readdir(directory, { withFileTypes: true });
  const actual = entries.map((entry) => entry.name).sort();
  const expected = expectedStagedFilenames(manifest, target);
  const matches =
    entries.every((entry) => entry.isFile()) &&
    actual.length === expected.length &&
    actual.every((value, index) => value === expected[index]);
  if (!matches) {
    throw new Error(
      `Bundled media-tools directory contains unexpected files: expected ${expected.join(', ')}, received ${actual.join(', ')}`,
    );
  }
}
