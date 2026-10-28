import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { cleanupNativeE2e, formatProcessLogs, readBootstrapCheckpoint } from './app-lifecycle.mjs';

const testRoot = 'owned-native-root';
const frontendBuildRoot = 'owned-native-build';
const child = { pid: 42 };

function harness({ failure, keepTemp = false, historicalExportEnabled = false, faults = {} } = {}) {
  const calls = [];
  const inputs = {
    app: { child },
    fixtureServer: {
      async close() {
        calls.push('server');
        if (faults.server) throw new Error('server close failed');
      },
    },
    testRoot,
    frontendBuildRoot,
    failure,
    keepTemp,
    historicalExportEnabled,
  };
  const adapters = {
    async terminateProcess(actual) {
      assert.equal(actual, child);
      calls.push('child');
      if (faults.child) throw new Error('child stop failed');
    },
    async remove(target, options) {
      calls.push(`remove:${target}`);
      assert.deepEqual(options, {
        recursive: true,
        force: true,
        maxRetries: 10,
        retryDelay: 200,
      });
      if (faults[target]) throw new Error(`${target} remove failed`);
    },
    exists(target) {
      calls.push(`exists:${target}`);
      return Boolean(faults[`left:${target}`]);
    },
    write(message) {
      calls.push(`write:${message}`);
    },
  };
  return { inputs, adapters, calls };
}

test('success closes owned child and server, then removes and verifies both roots', async () => {
  const { inputs, adapters, calls } = harness();
  assert.equal(await cleanupNativeE2e(inputs, adapters), undefined);
  assert.deepEqual(calls, [
    'child',
    'server',
    `remove:${testRoot}`,
    `exists:${testRoot}`,
    `remove:${frontendBuildRoot}`,
    `exists:${frontendBuildRoot}`,
  ]);
});

test('failure retains its cause and aggregates every independent cleanup fault', async () => {
  const failure = new Error('native scenario failed');
  const { inputs, adapters, calls } = harness({
    failure,
    faults: { child: true, server: true, [testRoot]: true, [frontendBuildRoot]: true },
  });
  const result = await cleanupNativeE2e(inputs, adapters);
  assert.equal(result.cause, failure);
  assert.match(result.message, /^native scenario failed\nCleanup failure: /);
  assert.match(
    result.message,
    /child stop failed; server close failed; owned-native-root remove failed; owned-native-build remove failed$/,
  );
  assert.deepEqual(calls, ['child', 'server', `remove:${testRoot}`, `remove:${frontendBuildRoot}`]);
});

test('failed post-removal verification still reaches frontend cleanup', async () => {
  const { inputs, adapters, calls } = harness({
    faults: { [`left:${testRoot}`]: true, [`left:${frontendBuildRoot}`]: true },
  });
  const result = await cleanupNativeE2e(inputs, adapters);
  assert.match(result.message, /^Native E2E cleanup failed\nCleanup failure: /);
  assert.match(result.message, /Native E2E sandbox was not removed: owned-native-root/);
  assert.match(result.message, /Native E2E frontend build was not removed: owned-native-build/);
  assert.equal(result.cause.message, `Native E2E sandbox was not removed: ${testRoot}`);
  assert.equal(calls.at(-1), `exists:${frontendBuildRoot}`);
});

for (const [name, options] of [
  ['explicit keep-temp', { keepTemp: true }],
  [
    'historical export failure',
    { failure: new Error('export failed'), historicalExportEnabled: true },
  ],
]) {
  test(`${name} retains only data root`, async () => {
    const { inputs, adapters, calls } = harness(options);
    assert.equal(await cleanupNativeE2e(inputs, adapters), options.failure);
    assert.deepEqual(calls, [
      'child',
      'server',
      `write:Native E2E data kept at ${testRoot}\n`,
      `remove:${frontendBuildRoot}`,
      `exists:${frontendBuildRoot}`,
    ]);
  });
}

test('partial process logs and bootstrap checkpoint remain in failure evidence', async () => {
  assert.equal(
    formatProcessLogs({ stdout: 'partial stdout ', stderr: ' partial stderr' }),
    '\nNative app stdout:\npartial stdout\nNative app stderr:\npartial stderr',
  );
  assert.equal(
    formatProcessLogs({ stdout: '', stderr: '' }),
    '\nNative app stdout:\n(empty)\nNative app stderr:\n(empty)',
  );
  const root = await fs.mkdtemp(path.join(os.tmpdir(), 'auralis-native-checkpoint-'));
  try {
    assert.equal(
      await readBootstrapCheckpoint(root),
      '\nNative E2E checkpoint trace: (not created)',
    );
    await fs.writeFile(path.join(root, 'native-e2e-bootstrap.txt'), 'partial checkpoint\n');
    assert.equal(
      await readBootstrapCheckpoint(root),
      '\nNative E2E checkpoint trace:\npartial checkpoint',
    );
  } finally {
    await fs.rm(root, { recursive: true, force: true });
  }
});
