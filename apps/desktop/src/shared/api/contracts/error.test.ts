import { describe, it, expect } from 'vitest';
import { isCommandError } from './error';

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
