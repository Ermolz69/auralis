import { describe, expect, it } from 'vitest';
import { formatClockTime } from './dateTime';

describe('formatClockTime', () => {
  it('formats valid timestamps with seconds', () => {
    expect(formatClockTime('2026-01-01T12:34:56')).toMatch(/^\d{2}:\d{2}:\d{2}$/);
  });

  it('uses a stable fallback for invalid timestamps', () => {
    expect(formatClockTime('not-a-date')).toBe('--:--:--');
    expect(formatClockTime('not-a-date', 'unknown')).toBe('unknown');
  });
});
