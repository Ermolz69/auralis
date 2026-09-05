import assert from 'node:assert/strict';
import test from 'node:test';
import {
  validateReleaseSigning,
  validateUpdaterSigning,
  windowsSigningConfig,
} from './signing-config.mjs';

const updaterSigning = {
  TAURI_SIGNING_PRIVATE_KEY: 'encrypted-private-key',
  TAURI_SIGNING_PRIVATE_KEY_PASSWORD: 'strong-password',
  AURALIS_UPDATER_PUBLIC_KEY: 'trusted-public-key',
};

test('accepts complete PFX and Azure signing configurations', () => {
  assert.equal(
    windowsSigningConfig({
      WINDOWS_SIGNING_MODE: 'pfx',
      WINDOWS_CERTIFICATE: 'base64',
      WINDOWS_CERTIFICATE_PASSWORD: 'password',
    }).mode,
    'pfx',
  );
  assert.equal(
    windowsSigningConfig({
      WINDOWS_SIGNING_MODE: 'azure',
      AZURE_CLIENT_ID: 'client',
      AZURE_CLIENT_SECRET: 'secret',
      AZURE_TENANT_ID: 'tenant',
      AZURE_ARTIFACT_SIGNING_ENDPOINT: 'endpoint',
      AZURE_ARTIFACT_SIGNING_ACCOUNT: 'account',
      AZURE_ARTIFACT_SIGNING_PROFILE: 'profile',
      AZURE_ARTIFACT_SIGNING_CLI_VERSION: '1.0.0',
    }).mode,
    'azure',
  );
});

test('rejects incomplete Windows and Apple signing configuration', () => {
  assert.throws(
    () => windowsSigningConfig({ WINDOWS_SIGNING_MODE: 'pfx' }),
    /WINDOWS_CERTIFICATE, WINDOWS_CERTIFICATE_PASSWORD/,
  );
  assert.throws(
    () =>
      validateReleaseSigning('macos', {
        ...updaterSigning,
        APPLE_CERTIFICATE: 'certificate',
      }),
    /APPLE_CERTIFICATE_PASSWORD, APPLE_ID, APPLE_PASSWORD, APPLE_TEAM_ID/,
  );
});

test('requires signed updater artifacts on every platform', () => {
  assert.doesNotThrow(() => validateReleaseSigning('linux', updaterSigning));
  assert.throws(
    () => validateReleaseSigning('linux', {}),
    /TAURI_SIGNING_PRIVATE_KEY, TAURI_SIGNING_PRIVATE_KEY_PASSWORD, AURALIS_UPDATER_PUBLIC_KEY/,
  );
  assert.throws(
    () => validateUpdaterSigning({ ...updaterSigning, TAURI_SIGNING_PRIVATE_KEY: 'CHANGE_ME' }),
    /placeholder values: TAURI_SIGNING_PRIVATE_KEY/,
  );
});
