// @vitest-environment jsdom
import { afterEach, expect, it, vi } from 'vitest';
import { installGlobalErrorReporting } from './globalErrorReporting';

afterEach(() => vi.restoreAllMocks());

it('reports bounded metadata without private messages, paths or rejection values', () => {
  const logger = vi.spyOn(console, 'error').mockImplementation(() => {});
  const stop = installGlobalErrorReporting();
  for (let i = 0; i < 30; i++) {
    window.dispatchEvent(
      new ErrorEvent('error', { message: 'SECRET C:/private.mp4 token=secret' }),
    );
    window.dispatchEvent(new Event('unhandledrejection'));
  }
  expect(logger).toHaveBeenCalledTimes(20);
  expect(JSON.stringify(logger.mock.calls)).not.toContain('SECRET');
  expect(JSON.stringify(logger.mock.calls)).not.toContain('private');
  stop();
  window.dispatchEvent(new Event('unhandledrejection'));
  expect(logger).toHaveBeenCalledTimes(20);
});
