// @vitest-environment jsdom
import { afterEach, expect, it, vi } from 'vitest';
import { installGlobalErrorReporting } from './globalErrorReporting';

afterEach(() => vi.restoreAllMocks());

it('keeps default error reporting available in debug policy', () => {
  vi.spyOn(console, 'error').mockImplementation(() => {});
  const stop = installGlobalErrorReporting(window, false);
  const error = new ErrorEvent('error', { cancelable: true });
  window.dispatchEvent(error);
  expect(error.defaultPrevented).toBe(false);
  stop();
});

it('reports bounded metadata without private messages, paths or rejection values', () => {
  const logger = vi.spyOn(console, 'error').mockImplementation(() => {});
  const stop = installGlobalErrorReporting(window, true);
  for (let i = 0; i < 30; i++) {
    const error = new ErrorEvent('error', {
      message: 'SECRET C:/private.mp4 token=secret',
      cancelable: true,
    });
    const rejection = new Event('unhandledrejection', { cancelable: true });
    window.dispatchEvent(error);
    window.dispatchEvent(rejection);
    expect(error.defaultPrevented).toBe(true);
    expect(rejection.defaultPrevented).toBe(true);
  }
  expect(logger).toHaveBeenCalledTimes(20);
  expect(JSON.stringify(logger.mock.calls)).not.toContain('SECRET');
  expect(JSON.stringify(logger.mock.calls)).not.toContain('private');
  stop();
  const late = new Event('unhandledrejection', { cancelable: true });
  window.dispatchEvent(late);
  expect(late.defaultPrevented).toBe(false);
  expect(logger).toHaveBeenCalledTimes(20);
});
