import assert from 'node:assert/strict';
import fs from 'node:fs';
import test from 'node:test';
import {
  assertAtomicReleaseWorkflow,
  collectMetadataErrors,
  collectTauriVersionErrors,
} from './check-release-metadata.mjs';

const valid = {
  workspaceCargo:
    '[workspace.package]\nversion = "0.1.0"\nedition = "2024"\nauthors = ["Auralis Contributors"]\n',
  memberCargos: [
    [
      'crate',
      '[package]\nname = "crate"\nversion.workspace = true\nedition.workspace = true\nauthors.workspace = true\n',
    ],
  ],
  rootPackage: { version: '0.1.0' },
  desktopPackage: { version: '0.1.0' },
  tauriConfig: { version: '../package.json', identifier: 'com.auralis.desktop' },
};

test('accepts synchronized inherited metadata', () => {
  assert.deepEqual(collectMetadataErrors(valid), []);
});

test('rejects placeholder authors, split versions and unsafe bundle identifiers', () => {
  const invalid = structuredClone(valid);
  invalid.workspaceCargo = invalid.workspaceCargo.replace('Auralis Contributors', 'you');
  invalid.desktopPackage.version = '0.0.0';
  invalid.tauriConfig.identifier = 'com.auralis.app';

  assert.deepEqual(collectMetadataErrors(invalid), [
    'workspace.package.authors must identify the project maintainers',
    'desktop package.json version must equal workspace.package.version',
    'Tauri identifier must be stable and must not end with .app',
  ]);
});

test('requires verified matrix artifacts before the only release write step', () => {
  const workflow = fs.readFileSync(
    new URL('../../.github/workflows/release.yml', import.meta.url),
    'utf8',
  );
  assert.doesNotThrow(() => assertAtomicReleaseWorkflow(workflow));
  assert.throws(
    () => assertAtomicReleaseWorkflow(workflow.replace('needs: build', 'needs: check')),
    /Publish job must wait for the complete build matrix/,
  );
  assert.throws(
    () =>
      assertAtomicReleaseWorkflow(
        workflow.replace(
          'uses: tauri-apps/tauri-action@v1',
          'uses: tauri-apps/tauri-action@v1\n        with:\n          tagName: app-v0.1.0',
        ),
      ),
    /Matrix build must not create a GitHub Release/,
  );
});

test('requires exact cross-language Tauri plugin versions and aligned core minors', () => {
  const versions = {
    workspaceCargo:
      '[workspace.dependencies]\ntauri = "2.11.5"\ntauri-plugin-dialog = "=2.7.3"\ntauri-plugin-process = "=2.3.1"\ntauri-plugin-updater = "=2.11.0"\n',
    tauriCargo:
      '[dependencies]\ntauri-plugin-dialog.workspace = true\ntauri-plugin-process = { workspace = true }\ntauri-plugin-updater.workspace = true\n',
    rootPackage: { devDependencies: { '@tauri-apps/cli': '2.11.4' } },
    desktopPackage: {
      dependencies: {
        '@tauri-apps/api': '2.11.1',
        '@tauri-apps/plugin-dialog': '2.7.3',
        '@tauri-apps/plugin-process': '2.3.1',
        '@tauri-apps/plugin-updater': '2.11.0',
      },
    },
  };

  assert.deepEqual(collectTauriVersionErrors(versions), []);

  const drifted = structuredClone(versions);
  drifted.desktopPackage.dependencies['@tauri-apps/plugin-dialog'] = '^2.7.3';
  drifted.desktopPackage.dependencies['@tauri-apps/api'] = '2.10.1';
  assert.deepEqual(collectTauriVersionErrors(drifted), [
    '@tauri-apps/api must use the same major/minor line as the tauri crate',
    '@tauri-apps/plugin-dialog must use an exact semantic version',
    'tauri-plugin-dialog must exactly match @tauri-apps/plugin-dialog',
  ]);
});
