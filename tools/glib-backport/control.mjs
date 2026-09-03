import assert from 'node:assert/strict';
import { stripVTControlCharacters } from 'node:util';

export function controlOutput(result) {
  return stripVTControlCharacters(`${result.stdout ?? ''}\n${result.stderr ?? ''}`);
}

export function verifyNegativeControl(result) {
  assert.ok(!result.error, result.error?.message);
  assert.ok(
    Number.isInteger(result.status) && result.status > 0,
    'The unpatched control must fail',
  );
  const output = controlOutput(result);
  assert.match(
    output,
    /Running unittests/,
    'A compile/setup failure is not evidence of the defect',
  );
  const crash = /signal: (6, SIGABRT|11, SIGSEGV)/.exec(output);
  assert.ok(crash, 'Expected the upstream invalid-pointer failure');
  return crash[0];
}
