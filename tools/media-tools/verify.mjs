import childProcess from 'node:child_process';
import fs from 'node:fs/promises';
import path from 'node:path';
import { fileMatches } from './integrity.mjs';
import { targetAssets } from './manifest.mjs';
import { renderNotices, renderProvenance } from './provenance.mjs';
import { verifyStagingDirectory } from './staged-files.mjs';

export async function verifyPrepared({ manifest, target, binariesDir, execute = true }) {
  await verifyStagingDirectory({ manifest, target, binariesDir });
  const verified = [];
  for (const asset of targetAssets(manifest, target)) {
    const executable = path.join(binariesDir, asset.output);
    if (!(await fileMatches(executable, asset.sha256))) {
      throw new Error(`Missing or invalid bundled ${asset.name}: ${executable}`);
    }
    if (process.platform !== 'win32') {
      const mode = (await fs.stat(executable)).mode;
      if ((mode & 0o111) === 0) throw new Error(`Bundled tool is not executable: ${executable}`);
    }
    if (execute) verifyVersion(executable, asset.tool.versionArgs, asset.tool.versionPattern);
    verified.push(asset.output);
  }

  for (const license of Object.values(manifest.licenses)) {
    const licensePath = path.join(binariesDir, license.output);
    if (!(await fileMatches(licensePath, license.sha256))) {
      throw new Error(`Missing or invalid bundled license: ${licensePath}`);
    }
    verified.push(license.output);
  }

  await verifyTextFile(
    path.join(binariesDir, 'media-tools-provenance.json'),
    renderProvenance(manifest, target),
  );
  await verifyTextFile(
    path.join(binariesDir, 'THIRD-PARTY-NOTICES.txt'),
    renderNotices(manifest, target),
  );
  return verified;
}

function verifyVersion(executable, versionArgs, versionPattern) {
  const result = childProcess.spawnSync(executable, versionArgs, {
    encoding: 'utf8',
    timeout: 30_000,
    windowsHide: true,
  });
  if (result.error) throw new Error(`Cannot execute bundled tool ${executable}: ${result.error.message}`);
  if (result.status !== 0) {
    throw new Error(`Bundled tool ${executable} exited with status ${result.status}`);
  }
  const output = `${result.stdout ?? ''}\n${result.stderr ?? ''}`;
  if (!new RegExp(versionPattern).test(output)) {
    throw new Error(`Bundled tool ${executable} did not report its pinned version`);
  }
}

async function verifyTextFile(filePath, expected) {
  let actual;
  try {
    actual = await fs.readFile(filePath, 'utf8');
  } catch (error) {
    if (error?.code === 'ENOENT') throw new Error(`Missing bundled provenance file: ${filePath}`);
    throw error;
  }
  if (actual !== expected) throw new Error(`Bundled provenance is stale: ${filePath}`);
}
