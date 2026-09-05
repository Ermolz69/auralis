export function windowsSigningConfig(environment = process.env) {
  const mode = environment.WINDOWS_SIGNING_MODE?.trim().toLowerCase();
  if (mode === 'pfx') {
    requireValues(environment, ['WINDOWS_CERTIFICATE', 'WINDOWS_CERTIFICATE_PASSWORD']);
    return {
      mode,
      timestampUrl: environment.WINDOWS_TIMESTAMP_URL?.trim() || 'http://timestamp.digicert.com',
    };
  }
  if (mode === 'azure') {
    requireValues(environment, [
      'AZURE_CLIENT_ID',
      'AZURE_CLIENT_SECRET',
      'AZURE_TENANT_ID',
      'AZURE_ARTIFACT_SIGNING_ENDPOINT',
      'AZURE_ARTIFACT_SIGNING_ACCOUNT',
      'AZURE_ARTIFACT_SIGNING_PROFILE',
      'AZURE_ARTIFACT_SIGNING_CLI_VERSION',
    ]);
    return { mode };
  }
  throw new Error('WINDOWS_SIGNING_MODE must be either "pfx" or "azure"');
}

export function validateReleaseSigning(platform, environment = process.env) {
  validateUpdaterSigning(environment);
  if (platform === 'windows') {
    windowsSigningConfig(environment);
    return;
  }
  if (platform === 'macos') {
    requireValues(environment, [
      'APPLE_CERTIFICATE',
      'APPLE_CERTIFICATE_PASSWORD',
      'APPLE_ID',
      'APPLE_PASSWORD',
      'APPLE_TEAM_ID',
    ]);
    return;
  }
  if (platform !== 'linux') throw new Error(`Unsupported release signing platform: ${platform}`);
}

export function validateUpdaterSigning(environment = process.env) {
  const names = [
    'TAURI_SIGNING_PRIVATE_KEY',
    'TAURI_SIGNING_PRIVATE_KEY_PASSWORD',
    'AURALIS_UPDATER_PUBLIC_KEY',
  ];
  requireValues(environment, names);
  const unsafe = names.filter((name) => /placeholder|change[_-]?me/i.test(environment[name]));
  if (unsafe.length > 0) {
    throw new Error(
      `Updater signing configuration contains placeholder values: ${unsafe.join(', ')}`,
    );
  }
}

function requireValues(environment, names) {
  const missing = names.filter((name) => !environment[name]?.trim());
  if (missing.length > 0) {
    throw new Error(`Missing required release signing configuration: ${missing.join(', ')}`);
  }
}
