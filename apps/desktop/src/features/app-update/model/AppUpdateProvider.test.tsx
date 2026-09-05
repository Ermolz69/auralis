// @vitest-environment jsdom
import { StrictMode } from 'react';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { AppUpdateCandidate, AppUpdateClient } from '../api/appUpdateClient';
import { AppUpdateProvider } from './AppUpdateProvider';
import { useAppUpdate } from './appUpdateContext';

afterEach(() => cleanup());

describe('AppUpdateProvider', () => {
  it('checks safely in StrictMode and reports that the app is current', async () => {
    const client = createClient(null);

    render(
      <StrictMode>
        <AppUpdateProvider client={client}>
          <UpdateProbe />
        </AppUpdateProvider>
      </StrictMode>,
    );

    await screen.findByText('upToDate');
    expect(client.check).toHaveBeenCalled();
  });

  it('downloads, verifies and installs an available update before relaunching', async () => {
    const candidate = createCandidate();
    const client = createClient(candidate);

    render(
      <AppUpdateProvider client={client}>
        <UpdateProbe />
      </AppUpdateProvider>,
    );

    await screen.findByText('available');
    expect(screen.getByText('0.2.0')).not.toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'Install' }));

    await screen.findByText('restarting');
    expect(screen.getByText('100')).not.toBeNull();
    expect(candidate.downloadAndInstall).toHaveBeenCalledOnce();
    expect(client.relaunch).toHaveBeenCalledOnce();
  });

  it('supports downloads whose total size is not reported', async () => {
    const candidate = createCandidate();
    vi.mocked(candidate.downloadAndInstall).mockImplementationOnce(async (onEvent) => {
      onEvent({ event: 'Started', data: {} });
      onEvent({ event: 'Progress', data: { chunkLength: 12 } });
      onEvent({ event: 'Finished' });
    });

    render(
      <AppUpdateProvider client={createClient(candidate)}>
        <UpdateProbe />
      </AppUpdateProvider>,
    );

    await screen.findByText('available');
    fireEvent.click(screen.getByRole('button', { name: 'Install' }));
    await screen.findByText('restarting');
    expect(screen.queryByText('100')).toBeNull();
  });

  it('keeps the installed version usable when a check fails', async () => {
    const client = createClient(null);
    vi.mocked(client.check).mockRejectedValueOnce(new Error('private network detail'));

    render(
      <AppUpdateProvider client={client}>
        <UpdateProbe />
      </AppUpdateProvider>,
    );

    await screen.findByText('error');
    expect(screen.getByText(/check your connection/i)).not.toBeNull();
    expect(screen.queryByText(/private network detail/i)).toBeNull();
  });

  it('keeps the current app usable when installation fails', async () => {
    const candidate = createCandidate();
    vi.mocked(candidate.downloadAndInstall).mockRejectedValueOnce(new Error('signature mismatch'));
    const client = createClient(candidate);

    render(
      <AppUpdateProvider client={client}>
        <UpdateProbe />
      </AppUpdateProvider>,
    );

    await screen.findByText('available');
    fireEvent.click(screen.getByRole('button', { name: 'Install' }));

    await screen.findByText('error');
    expect(screen.getByText(/this version will continue to work/i)).not.toBeNull();
    expect(candidate.close).toHaveBeenCalledOnce();
    expect(client.relaunch).not.toHaveBeenCalled();
  });

  it('closes a superseded updater resource during a manual recheck', async () => {
    const candidate = createCandidate();
    const client = createClient(candidate);
    vi.mocked(client.check).mockResolvedValueOnce(candidate).mockResolvedValueOnce(null);

    render(
      <AppUpdateProvider client={client}>
        <UpdateProbe />
      </AppUpdateProvider>,
    );

    await screen.findByText('available');
    fireEvent.click(screen.getByRole('button', { name: 'Check' }));
    await screen.findByText('upToDate');
    expect(candidate.close).toHaveBeenCalledOnce();
  });

  it('does not contact the release endpoint outside an installed Tauri build', async () => {
    const client = createClient(null, false);

    render(
      <AppUpdateProvider client={client}>
        <UpdateProbe />
      </AppUpdateProvider>,
    );

    await screen.findByText('unsupported');
    expect(client.check).not.toHaveBeenCalled();
  });
});

function UpdateProbe() {
  const update = useAppUpdate();
  return (
    <div>
      <span>{update.phase}</span>
      <span>{update.update?.version}</span>
      <span>{update.progress?.percent}</span>
      <span>{update.error}</span>
      <button type="button" onClick={() => void update.installUpdate()}>
        Install
      </button>
      <button type="button" onClick={() => void update.checkForUpdates()}>
        Check
      </button>
    </div>
  );
}

function createCandidate(): AppUpdateCandidate {
  return {
    currentVersion: '0.1.0',
    version: '0.2.0',
    date: '2026-09-05T12:00:00Z',
    body: 'Release notes',
    downloadAndInstall: vi.fn(async (onEvent) => {
      onEvent({ event: 'Started', data: { contentLength: 100 } });
      onEvent({ event: 'Progress', data: { chunkLength: 40 } });
      onEvent({ event: 'Progress', data: { chunkLength: 60 } });
      onEvent({ event: 'Finished' });
    }),
    close: vi.fn(async () => undefined),
  };
}

function createClient(candidate: AppUpdateCandidate | null, supported = true): AppUpdateClient {
  return {
    isSupported: () => supported,
    getCurrentVersion: vi.fn(async () => '0.1.0'),
    check: vi.fn(async () => candidate),
    relaunch: vi.fn(async () => undefined),
  };
}
