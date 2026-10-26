import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';

export function sourceCues(srt) {
  const stamp = (value) => {
    const match = /^(\d{2,}):([0-5]\d):([0-5]\d),(\d{3})$/.exec(value);
    assert.ok(match, 'Invalid frozen SRT timestamp');
    return (
      ((Number(match[1]) * 60 + Number(match[2])) * 60 + Number(match[3])) * 1000 + Number(match[4])
    );
  };
  const cues = srt
    .replace(/^\uFEFF/, '')
    .replaceAll('\r\n', '\n')
    .replace(/^\n+|\n+$/g, '')
    .split(/\n[ \t]*\n/)
    .map((block, ordinal) => {
      const lines = block.split('\n');
      assert.match(lines[0], /^\d+$/, 'Frozen SRT requires numeric cue labels');
      const time = /^(\S+)\s+-->\s+(\S+)\s*$/.exec(lines[1]);
      assert.ok(time && lines.length > 2, 'Invalid frozen SRT cue');
      const start_ms = stamp(time[1]),
        end_ms = stamp(time[2]);
      assert.ok(end_ms > start_ms && lines.slice(2).join('\n').trim());
      return {
        id: `cue-${ordinal + 1}`,
        label: lines[0],
        start_ms,
        end_ms,
        text: lines.slice(2).join('\n'),
      };
    });
  assert.ok(cues.length > 0);
  return cues;
}

export function timingDigest(cues) {
  return createHash('sha256')
    .update(JSON.stringify(cues.map((c) => [c.label, c.start_ms, c.end_ms])))
    .digest('hex');
}

export function verifyCueCoverage(cues, source) {
  assert.equal(new Set(cues.map((c) => c.id)).size, cues.length, 'Duplicate approved cue');
  assert.deepEqual(
    cues.map((c) => [c.id, c.start_ms, c.end_ms]),
    source.map((c) => [c.id, c.start_ms, c.end_ms]),
    'Approved speech lost, reordered or retimed original source cues',
  );
}
