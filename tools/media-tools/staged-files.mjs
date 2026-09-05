import fs from 'node:fs/promises';
import path from 'node:path';
import { targetAssets, toolNames } from './manifest.mjs';

const metadataFilenames = ['media-tools-provenance.json', 'THIRD-PARTY-NOTICES.txt'];

export function expectedStagedFilenames(manifest, target) {
  return [
    ...targetAssets(manifest, target).map((asset) => asset.output),
    ...Object.values(manifest.licenses).map((license) => license.output),
    ...metadataFilenames,
  ].sort();
}

export async function prepareStagingDirectory({ manifest, target, binariesDir }) {
  await fs.mkdir(binariesDir, { recursive: true });
  const expected = new Set(expectedStagedFilenames(manifest, target));

  for (const entry of await fs.readdir(binariesDir, { withFileTypes: true })) {
    const candidate = path.join(binariesDir, entry.name);
    if (entry.isFile() && expected.has(entry.name)) continue;
    if (entry.isFile() && isKnownGeneratedFile(manifest, entry.name)) {
      await fs.rm(candidate, { force: true });
      continue;
    }
    throw new Error(`Unexpected entry in media-tools staging directory: ${candidate}`);
  }
}

export async function verifyStagingDirectory({ manifest, target, binariesDir }) {
  const expected = expectedStagedFilenames(manifest, target);
  const entries = await fs.readdir(binariesDir, { withFileTypes: true });
  const actual = entries.map((entry) => entry.name).sort();
  if (entries.some((entry) => !entry.isFile()) || !sameList(actual, expected)) {
    throw new Error(
      `Media-tools staging directory is not deterministic: expected ${expected.join(', ')}, received ${actual.join(', ')}`,
    );
  }
}

function isKnownGeneratedFile(manifest, filename) {
  return (
    toolNames.some((tool) => filename.startsWith(`${tool}-`)) ||
    Object.values(manifest.licenses).some((license) => license.output === filename) ||
    metadataFilenames.includes(filename)
  );
}

function sameList(left, right) {
  return left.length === right.length && left.every((value, index) => value === right[index]);
}
