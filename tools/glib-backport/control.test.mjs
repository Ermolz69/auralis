import assert from 'node:assert/strict';
import { test } from 'node:test';
import { controlOutput, verifyNegativeControl } from './control.mjs';

const plain = {
  status: 101,
  stdout: 'running 1 test\n',
  stderr:
    'Running unittests src/lib.rs\nprocess exited (signal: 11, SIGSEGV: invalid memory reference)',
};

test('accepts the actual pointer crash with plain or CI-colored Cargo output', () => {
  const colored = {
    ...plain,
    stderr: plain.stderr.replace('Running', '\x1b[1m\x1b[92mRunning\x1b[0m'),
  };
  assert.equal(controlOutput(colored), controlOutput(plain));
  assert.equal(verifyNegativeControl(plain), 'signal: 11, SIGSEGV');
  assert.equal(verifyNegativeControl(colored), 'signal: 11, SIGSEGV');
  assert.equal(
    verifyNegativeControl({ ...plain, stderr: plain.stderr.replace('11, SIGSEGV', '6, SIGABRT') }),
    'signal: 6, SIGABRT',
  );
});

test('never treats build/setup errors or a successful control as a reproduced defect', () => {
  for (const result of [
    { ...plain, status: 0 },
    { ...plain, status: null },
    { ...plain, error: new Error('spawn failed') },
    { ...plain, stderr: 'error: compilation failed; SIGSEGV appears in a source diagnostic' },
    { ...plain, stderr: 'Running unittests src/lib.rs\nassertion failed' },
    { ...plain, stderr: plain.stderr.replace('11, SIGSEGV', '9, SIGKILL') },
    { status: 1 },
  ]) {
    assert.throws(() => verifyNegativeControl(result));
  }
});
