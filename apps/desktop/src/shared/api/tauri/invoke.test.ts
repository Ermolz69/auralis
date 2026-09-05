import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import { invoke } from './invoke';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('typed Tauri invoke boundary', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('forwards a typed command and returns its result', async () => {
    vi.mocked(tauriInvoke).mockResolvedValueOnce('healthy');

    await expect(invoke('health_check')).resolves.toBe('healthy');
    expect(tauriInvoke).toHaveBeenCalledExactlyOnceWith('health_check', undefined);
  });

  it('keeps only the public fields of a known command error', async () => {
    vi.mocked(tauriInvoke).mockRejectedValueOnce({
      code: 'BUSY',
      message: 'The project is busy',
      internalContext: 'must not escape',
    });

    await expect(invoke('health_check')).rejects.toEqual({
      code: 'BUSY',
      message: 'The project is busy',
    });
  });

  it('sanitizes unexpected backend failures', async () => {
    vi.mocked(tauriInvoke).mockRejectedValueOnce(
      new Error('C:\\Users\\person\\private.mp4 token=secret'),
    );

    await expect(invoke('health_check')).rejects.toEqual({
      code: 'INTERNAL',
      message: 'An unexpected system error occurred',
    });
  });

  it('rejects an incompatible success payload at runtime', async () => {
    vi.mocked(tauriInvoke).mockResolvedValueOnce({ status: 'healthy', secret: 'do-not-log' });

    await expect(invoke('health_check')).rejects.toThrow(
      'Invalid payload received for IPC command "health_check"',
    );
  });
});
