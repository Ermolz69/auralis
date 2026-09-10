import assert from 'node:assert/strict';
import test from 'node:test';
import { collectRunnerImageErrors } from './check-runner-images.mjs';

const validPolicy = {
  schemaVersion: 1,
  source: 'https://github.com/actions/runner-images#available-images',
  reviewedAt: '2026-09-06',
  pins: {
    windows: ['windows-2025'],
    macos: ['macos-26'],
  },
};

const workflows = (windows = 'windows-2025', macos = 'macos-26') => [
  [
    'desktop.yml',
    `jobs:\n  build:\n    strategy:\n      matrix:\n        platform:\n          - ${windows}\n          - ${macos}\n    runs-on: \${{ matrix.platform }}\n`,
  ],
];

test('accepts only reviewed concrete runner labels', () => {
  assert.deepEqual(collectRunnerImageErrors({ policy: validPolicy, workflows: workflows() }), []);
});

test('rejects floating latest aliases before GitHub can migrate them', () => {
  const errors = collectRunnerImageErrors({
    policy: validPolicy,
    workflows: workflows('windows-latest', 'macos-latest'),
  });
  assert.ok(errors.includes('desktop.yml: floating runner label is forbidden: windows-latest'));
  assert.ok(errors.includes('desktop.yml: floating runner label is forbidden: macos-latest'));
});

test('rejects unreviewed concrete image changes', () => {
  const errors = collectRunnerImageErrors({
    policy: validPolicy,
    workflows: workflows('windows-2022', 'macos-15'),
  });
  assert.ok(errors.some((error) => error.includes('windows-2022 is not reviewed')));
  assert.ok(errors.some((error) => error.includes('macos-15 is not reviewed')));
});

test('rejects incomplete or floating policy definitions', () => {
  const policy = structuredClone(validPolicy);
  policy.pins.windows = ['windows-latest'];
  delete policy.reviewedAt;
  assert.deepEqual(collectRunnerImageErrors({ policy, workflows: workflows() }), [
    'reviewedAt must use YYYY-MM-DD',
    'pins.windows contains an invalid concrete label: windows-latest',
  ]);
});
