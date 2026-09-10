import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { AppUpdateContext, type AppUpdateContextValue } from '../model/appUpdateContext';
import { AppUpdatePanel } from './AppUpdatePanel';

afterEach(() => cleanup());

describe('AppUpdatePanel', () => {
  it('presents an available signed release and starts its installation', () => {
    const installUpdate = vi.fn(async () => undefined);
    renderPanel({
      phase: 'available',
      update: {
        version: '0.2.0',
        date: '2026-09-05T12:00:00Z',
        notes: 'Safer cancellation and recovery.',
      },
      installUpdate,
    });

    expect(screen.getByText('Update available')).not.toBeNull();
    expect(screen.getByText('Version 0.2.0')).not.toBeNull();
    expect(screen.getByText('Safer cancellation and recovery.')).not.toBeNull();
    expect(screen.getByRole('time').getAttribute('datetime')).toBe('2026-09-05T12:00:00Z');

    fireEvent.click(screen.getByRole('button', { name: 'Download and install 0.2.0' }));

    expect(installUpdate).toHaveBeenCalledOnce();
  });

  it('announces determinate download progress and prevents duplicate checks', () => {
    const checkForUpdates = vi.fn(async () => undefined);
    renderPanel({
      phase: 'downloading',
      progress: { downloadedBytes: 42, totalBytes: 100, percent: 42 },
      checkForUpdates,
    });

    const action = screen.getByRole<HTMLButtonElement>('button', {
      name: 'Installing update…',
    });
    const progress = screen.getByRole('progressbar', { name: 'Update download progress' });

    expect(action.disabled).toBe(true);
    expect(progress.getAttribute('aria-valuenow')).toBe('42');
    expect(progress.getAttribute('aria-valuetext')).toBe('42%');
    fireEvent.click(action);
    expect(checkForUpdates).not.toHaveBeenCalled();
  });

  it('uses an indeterminate progress contract when total size is unknown', () => {
    renderPanel({
      phase: 'downloading',
      progress: { downloadedBytes: 64 },
    });

    const progress = screen.getByRole('progressbar', { name: 'Update download progress' });

    expect(screen.getByText('…%')).not.toBeNull();
    expect(progress.getAttribute('aria-busy')).toBe('true');
    expect(progress.getAttribute('aria-valuenow')).toBeNull();
  });

  it('keeps update recovery actions accessible across terminal panel states', () => {
    const checkForUpdates = vi.fn(async () => undefined);
    const { rerender } = renderPanel({ phase: 'upToDate', checkForUpdates });

    expect(screen.getByRole('status').textContent).toContain(
      'You are using the latest published version.',
    );
    fireEvent.click(screen.getByRole('button', { name: 'Check for updates' }));
    expect(checkForUpdates).toHaveBeenCalledOnce();

    rerender(panel({ phase: 'unsupported', checkForUpdates }));
    expect(screen.getByText(/signed installed builds of Auralis/i)).not.toBeNull();
    expect(
      screen.getByRole<HTMLButtonElement>('button', { name: 'Check for updates' }).disabled,
    ).toBe(true);

    rerender(
      panel({
        phase: 'error',
        error: 'Could not verify the release signature.',
        checkForUpdates,
      }),
    );
    expect(screen.getByRole('alert').textContent).toContain(
      'Could not verify the release signature.',
    );
    expect(
      screen.getByRole<HTMLButtonElement>('button', { name: 'Check for updates' }).disabled,
    ).toBe(false);
  });
});

const defaultValue: AppUpdateContextValue = {
  phase: 'idle',
  currentVersion: '0.1.0',
  update: null,
  progress: null,
  error: null,
  checkForUpdates: async () => undefined,
  installUpdate: async () => undefined,
};

function panel(overrides: Partial<AppUpdateContextValue>) {
  return (
    <AppUpdateContext.Provider value={{ ...defaultValue, ...overrides }}>
      <AppUpdatePanel />
    </AppUpdateContext.Provider>
  );
}

function renderPanel(overrides: Partial<AppUpdateContextValue>) {
  return render(panel(overrides));
}
