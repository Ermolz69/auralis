import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { verifyBundledApplication } from './bundled-application.mjs';
import { recordBundledApplication } from '../release/record-bundled-application.mjs';
import { prepareBundledApplication } from '../release/prepare-bundled-application.mjs';

test('the packaged application must match the current executable, including same-size changes', async (context) => {
  const temporary = await fs.mkdtemp(path.join(os.tmpdir(), 'auralis-bundled-app-test-'));
  context.after(() => fs.rm(temporary, { recursive: true, force: true }));
  const expected = path.join(temporary, 'auralis-app.exe');
  const root = path.join(temporary, 'extracted');
  const application = path.join(root, 'Auralis', path.basename(expected));
  await fs.mkdir(path.dirname(application), { recursive: true });
  const source = 'current binary __TAURI_BUNDLE_TYPE_VAR_UNK';
  const packaged = source.replace('VAR_UNK', 'VAR_MSI');
  await fs.writeFile(expected, source);
  await prepareBundledApplication(expected);
  await fs.writeFile(expected, packaged);
  await recordBundledApplication(expected, () => {});
  await fs.writeFile(expected, source);
  await assert.rejects(verifyBundledApplication(root, expected), /found 0/);
  await fs.writeFile(application, packaged);
  const report = await verifyBundledApplication(root, expected);
  assert.equal(report.bytes, Buffer.byteLength(packaged));
  assert.match(report.sha256, /^[a-f0-9]{64}$/u);
  await fs.writeFile(application, packaged.replace('current', 'changed'));
  await assert.rejects(verifyBundledApplication(root, expected), /differs from the captured/);
  await fs.writeFile(application, packaged);
  await fs.writeFile(expected, source.replace('current', 'changed'));
  await assert.rejects(verifyBundledApplication(root, expected), /record differs from the current/);
  await fs.writeFile(expected, source);
  await fs.copyFile(application, path.join(root, path.basename(expected)));
  await assert.rejects(verifyBundledApplication(root, expected), /found 2/);
});

test('records the complete post-signing bytes without accepting a failed or ambiguous capture', async (context) => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), 'auralis-bundled-sign-test-'));
  context.after(() => fs.rm(root, { recursive: true, force: true }));
  const expected = path.join(root, 'auralis-app.exe');
  const extracted = path.join(root, 'extracted');
  const application = path.join(extracted, 'auralis-app.exe');
  const source = 'literal __TAURI_BUNDLE_TYPE_VAR_NSS binary __TAURI_BUNDLE_TYPE_VAR_UNK';
  const packaged = source.replace('VAR_UNK', 'VAR_MSI');
  await fs.mkdir(extracted);
  await fs.writeFile(expected, source);
  await prepareBundledApplication(expected);
  await fs.writeFile(expected, packaged);
  await recordBundledApplication(expected, () => fs.appendFile(expected, 'signature bytes'));
  await fs.copyFile(expected, application);
  await fs.writeFile(expected, source);
  await verifyBundledApplication(extracted, expected);
  const recordFile = path.join(root, 'bundle-application-digests/msi.json');
  const record = await fs.readFile(recordFile);
  await fs.writeFile(expected, packaged);
  await assert.rejects(
    recordBundledApplication(expected, () => {
      throw new Error('signing failed');
    }),
    /signing failed/,
  );
  assert.deepEqual(await fs.readFile(recordFile), record);
  await fs.writeFile(expected, `${packaged}${packaged}`);
  await assert.rejects(
    recordBundledApplication(expected, () => {}),
    /Invalid pre-package/,
  );
  await fs.writeFile(expected, source);
  await recordBundledApplication(expected, () => {});
  assert.deepEqual(await fs.readFile(recordFile), record);
  await fs.rm(recordFile);
  await assert.rejects(verifyBundledApplication(extracted, expected), /ENOENT/);
});

test('captures only the unpatched variable and rejects changes before the signer runs', async (context) => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), 'auralis-bundle-baseline-test-'));
  context.after(() => fs.rm(root, { recursive: true, force: true }));
  const executable = path.join(root, 'auralis-app.exe');
  const source = 'unchanged __TAURI_BUNDLE_TYPE_VAR_UNK';
  await fs.writeFile(executable, source.replace('VAR_UNK', 'VAR_MSI'));
  await assert.rejects(prepareBundledApplication(executable), /unpatched Tauri/);
  await fs.writeFile(executable, `${source}${source}`);
  await assert.rejects(prepareBundledApplication(executable), /exactly one/);
  await fs.writeFile(executable, source);
  await prepareBundledApplication(executable);
  await fs.writeFile(
    executable,
    source.replace('unchanged', 'altered!!').replace('VAR_UNK', 'VAR_MSI'),
  );
  let called = false;
  await assert.rejects(
    recordBundledApplication(executable, () => {
      called = true;
    }),
    /changed after pre-package/,
  );
  assert.equal(called, false);
});
