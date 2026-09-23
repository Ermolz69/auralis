import { listen as tauriListen } from '@tauri-apps/api/event';
import { describe, expect, it, vi } from 'vitest';
import { listen } from './listen';

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}));

describe('typed Tauri event boundary', () => {
  it('validates an event before forwarding it', async () => {
    const handler = vi.fn();
    vi.mocked(tauriListen).mockImplementationOnce(async (_event, callback) => {
      callback({ event: 'project-updated', id: 1, payload: { projectId: 'project-1' } });
      return () => undefined;
    });

    await listen('project-updated', handler);

    expect(handler).toHaveBeenCalledWith({
      event: 'project-updated',
      id: 1,
      payload: { projectId: 'project-1' },
    });
  });

  it('rejects an incompatible event payload without throwing from the async listener', async () => {
    const handler = vi.fn();
    const onInvalidPayload = vi.fn();
    vi.mocked(tauriListen).mockImplementationOnce(async (_event, callback) => {
      expect(() => callback({ event: 'project-updated', id: 1, payload: {} })).not.toThrow();
      return () => undefined;
    });

    await listen('project-updated', handler, { onInvalidPayload });

    expect(handler).not.toHaveBeenCalled();
    expect(onInvalidPayload).toHaveBeenCalledExactlyOnceWith(
      expect.objectContaining({
        message: 'Invalid payload received for IPC event "project-updated"',
      }),
    );
  });

  it('logs a payload contract failure safely when no recovery handler is provided', async () => {
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => undefined);
    vi.mocked(tauriListen).mockImplementationOnce(async (_event, callback) => {
      callback({ event: 'project-updated', id: 1, payload: { secret: 'not-forwarded' } });
      return () => undefined;
    });

    await listen('project-updated', vi.fn());

    expect(consoleError).toHaveBeenCalledExactlyOnceWith(
      'Rejected invalid Tauri event payload',
      expect.objectContaining({ name: 'IpcContractError' }),
    );
    expect(JSON.stringify(consoleError.mock.calls)).not.toContain('not-forwarded');
    consoleError.mockRestore();
  });

  it('normalizes listener registration failures like command failures', async () => {
    vi.mocked(tauriListen).mockRejectedValueOnce(new Error('private filesystem path'));

    await expect(listen('project-updated', vi.fn())).rejects.toEqual({
      code: 'INTERNAL',
      message: 'An unexpected system error occurred',
    });
  });

  it('keeps only public fields from a known listener failure', async () => {
    vi.mocked(tauriListen).mockRejectedValueOnce({
      code: 'BUSY',
      message: 'Try again',
      internalContext: 'private',
    });

    await expect(listen('project-updated', vi.fn())).rejects.toEqual({
      code: 'BUSY',
      message: 'Try again',
    });
  });
});
