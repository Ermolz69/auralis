import assert from 'node:assert/strict';
import fs from 'node:fs';
import test from 'node:test';
import { assertAtomicReleaseWorkflow, collectMetadataErrors } from './check-release-metadata.mjs';

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
