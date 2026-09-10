// @vitest-environment jsdom
import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { AppErrorBoundary } from './AppErrorBoundary';
import { reportReactError } from './reactErrorReporting';

function BrokenScreen(): never {
  throw new Error('C:\\Users\\person\\private.mp4 token=secret');
}

describe('AppErrorBoundary', () => {
  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
  });

  it('replaces a failed React tree with a recoverable, non-sensitive screen', () => {
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => undefined);

    render(
      <AppErrorBoundary>
        <BrokenScreen />
      </AppErrorBoundary>,
    );

    expect(screen.getByRole('alert').textContent).toContain("Auralis couldn't display this screen");
    expect(
      screen.getByRole<HTMLButtonElement>('button', { name: 'Restart application' }).disabled,
    ).toBe(false);
    expect(screen.queryByText(/private\.mp4|token=secret/i)).toBeNull();

    reportReactError('caught', new Error('C:\\Users\\person\\private.mp4 token=secret'));
    const safeLog = consoleError.mock.calls.find(
      ([message]) => message === 'Auralis UI caught error',
    );
    expect(safeLog).toEqual([
      'Auralis UI caught error',
      { code: 'INTERNAL', message: 'An unexpected system error occurred' },
    ]);
  });
});
