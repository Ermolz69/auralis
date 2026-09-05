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

  it('rejects an incompatible event payload', async () => {
    vi.mocked(tauriListen).mockImplementationOnce(async (_event, callback) => {
      expect(() => callback({ event: 'project-updated', id: 1, payload: {} })).toThrow(
        'Invalid payload received for IPC event "project-updated"',
      );
      return () => undefined;
    });

    await listen('project-updated', vi.fn());
  });
});
