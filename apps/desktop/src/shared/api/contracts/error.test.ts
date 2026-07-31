import { describe, it, expect } from 'vitest';
import { isCommandError, toCommandError } from './error';

describe('isCommandError', () => {
  it('returns true for valid NOT_FOUND error', () => {
    expect(isCommandError({ code: 'NOT_FOUND', message: 'test' })).toBe(true);
  });

  it('returns true for valid CONFLICT error', () => {
    expect(isCommandError({ code: 'CONFLICT', message: 'test' })).toBe(true);
  });

  it('returns true for valid BUSY error', () => {
    expect(isCommandError({ code: 'BUSY', message: 'test' })).toBe(true);
  });

  it('returns false for invalid code', () => {
    expect(isCommandError({ code: 'INVALID_CODE', message: 'test' })).toBe(false);
  });

  it('returns false for missing message', () => {
    expect(isCommandError({ code: 'NOT_FOUND' })).toBe(false);
  });

  it('returns false for non-object', () => {
    expect(isCommandError('error string')).toBe(false);
    expect(isCommandError(null)).toBe(false);
  });
});

describe('toCommandError', () => {
  it('returns a clean copy of valid CommandError and drops extra fields', () => {
    const raw = { code: 'BUSY' as const, message: 'Busy message', debug: 'SECRET' };
    const res = toCommandError(raw);
    expect(res).toEqual({ code: 'BUSY', message: 'Busy message' });
    expect((res as any).debug).toBeUndefined();
  });

  it('returns generic INTERNAL for string errors', () => {
    const res = toCommandError('C:\\Users\\secret\\video.mp4 token=SECRET Bearer token');
    expect(res).toEqual({ code: 'INTERNAL', message: 'An unexpected system error occurred' });
  });

  it('returns generic INTERNAL for generic Error objects', () => {
    const res = toCommandError(new Error('sensitive DB error'));
    expect(res).toEqual({ code: 'INTERNAL', message: 'An unexpected system error occurred' });
  });

  it('returns generic INTERNAL for malformed objects', () => {
    const res = toCommandError({ code: 'NOT_FOUND', msg: 'wrong field' });
    expect(res).toEqual({ code: 'INTERNAL', message: 'An unexpected system error occurred' });
  });
});
