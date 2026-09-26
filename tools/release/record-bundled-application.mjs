import crypto from 'node:crypto';
import fs from 'node:fs/promises';
import path from 'node:path';

const APPLICATION_NAME = 'auralis-app.exe';
const MARKER_PREFIX = '__TAURI_BUNDLE_TYPE_VAR_';
const SOURCE_MARKER = Buffer.from(`${MARKER_PREFIX}UNK`);
const PACKAGE_MARKERS = new Map([
  ['MSI', 'msi'],
  ['NSS', 'nsis'],
]);
const digest = (bytes) => crypto.createHash('sha256').update(bytes).digest('hex');

export async function recordBundledApplication(target, sign) {
  if (path.basename(target) !== APPLICATION_NAME) return sign();
  const beforeSigning = await fs.readFile(target);
  const directory = path.join(path.dirname(target), 'bundle-application-digests');
  const baseline = JSON.parse(await fs.readFile(path.join(directory, 'source.json'), 'utf8'));
  const markerIndex = baseline.marker_offset;
  if (
    baseline.schema_version !== 1 ||
    baseline.application_file !== APPLICATION_NAME ||
    !Number.isSafeInteger(markerIndex) ||
    markerIndex < 0 ||
    markerIndex + SOURCE_MARKER.length > beforeSigning.length ||
    baseline.source_bytes !== beforeSigning.length ||
    !/^[a-f0-9]{64}$/u.test(baseline.source_sha256) ||
    beforeSigning.subarray(markerIndex, markerIndex + MARKER_PREFIX.length).toString('ascii') !==
      MARKER_PREFIX
  ) {
    throw new Error('Invalid pre-package application identity');
  }
  const kind = beforeSigning
    .subarray(markerIndex + MARKER_PREFIX.length, markerIndex + SOURCE_MARKER.length)
    .toString('ascii');
  const packageType = PACKAGE_MARKERS.get(kind);
  if (kind !== 'UNK' && !packageType)
    throw new Error(`Unsupported application bundle marker: ${kind}`);
  const source = Buffer.from(beforeSigning);
  SOURCE_MARKER.copy(source, markerIndex);
  if (digest(source) !== baseline.source_sha256)
    throw new Error('Application changed after pre-package identity capture');
  await sign();
  if (!packageType) return;
  const packaged = await fs.readFile(target);
  await fs.mkdir(directory, { recursive: true });
  const report = {
    schema_version: 1,
    created_at: new Date().toISOString(),
    package_type: packageType,
    application_file: APPLICATION_NAME,
    source_sha256: baseline.source_sha256,
    packaged_sha256: digest(packaged),
    packaged_bytes: packaged.length,
  };
  const destination = path.join(directory, `${packageType}.json`);
  const temporary = `${destination}.${process.pid}.tmp`;
  try {
    await fs.writeFile(temporary, `${JSON.stringify(report, null, 2)}\n`);
    await fs.rename(temporary, destination);
  } finally {
    await fs.rm(temporary, { force: true });
  }
}
