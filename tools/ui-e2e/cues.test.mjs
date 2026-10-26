import assert from 'node:assert/strict';
import { test } from 'node:test';
import { sourceCues, timingDigest, verifyCueCoverage } from './cues.mjs';

const srt = '7\n00:00:00,100 --> 00:00:01,000\n你好\n\n9\n00:00:01,500 --> 00:00:03,200\n世界\n';
test('independent SRT coverage detects matching but truncated approval and plan', () => {
  const original = sourceCues(srt);
  verifyCueCoverage(original, original);
  for (const cues of [
    original.slice(0, 1),
    [...original].reverse(),
    [original[0], original[0]],
    original.map((c) => ({ ...c, end_ms: c.end_ms + 1 })),
  ])
    assert.throws(() => verifyCueCoverage(cues, original));
  assert.equal(timingDigest(original), timingDigest(sourceCues(srt.replaceAll('\n', '\r\n'))));
  assert.notEqual(
    timingDigest(original),
    timingDigest(sourceCues(srt.replace('00:00:03,200', '00:00:03,201'))),
  );
});

test('invalid or empty subtitle input never establishes coverage', () => {
  for (const value of [
    '',
    '1\n00:00:02,000 --> 00:00:01,000\ntext',
    '1\n00:00:00,000 --> 00:00:01,000',
  ])
    assert.throws(() => sourceCues(value));
});
