import fs from 'node:fs/promises';
import path from 'node:path';
import { sha256File } from './integrity.mjs';

export async function verifyBundledApplication(root, expectedExecutable, packageType = 'msi') {
  if (!['msi', 'nsis'].includes(packageType)) throw new Error('Unsupported Windows package type');
  const name = path.basename(expectedExecutable);
  const recordFile = path.join(
    path.dirname(expectedExecutable),
    'bundle-application-digests',
    `${packageType}.json`,
  );
  const record = JSON.parse(await fs.readFile(recordFile, 'utf8'));
  if (
    record.schema_version !== 1 ||
    record.package_type !== packageType ||
    record.application_file !== name ||
    !/^[a-f0-9]{64}$/u.test(record.source_sha256) ||
    !/^[a-f0-9]{64}$/u.test(record.packaged_sha256) ||
    !Number.isSafeInteger(record.packaged_bytes) ||
    record.packaged_bytes <= 0
  ) {
    throw new Error('Invalid bundled application digest record');
  }
  if (record.source_sha256 !== (await sha256File(expectedExecutable))) {
    throw new Error('Bundled application record differs from the current release executable');
  }
  const matches = [];
  async function visit(directory) {
    for (const entry of await fs.readdir(directory, { withFileTypes: true })) {
      const file = path.join(directory, entry.name);
      if (entry.isDirectory()) await visit(file);
      else if (entry.name === name) {
        if (!entry.isFile()) throw new Error(`Bundled application must be a regular file: ${name}`);
        matches.push(file);
      }
    }
  }
  await visit(root);
  if (matches.length !== 1) {
    throw new Error(`Expected exactly one bundled application ${name}, found ${matches.length}`);
  }
  const actualSha256 = await sha256File(matches[0]);
  const bytes = (await fs.stat(matches[0])).size;
  if (actualSha256 !== record.packaged_sha256 || bytes !== record.packaged_bytes) {
    throw new Error('Bundled application differs from the captured package executable');
  }
  return {
    file: path.relative(root, matches[0]),
    bytes,
    sha256: actualSha256,
    sourceSha256: record.source_sha256,
    packageType,
  };
}
