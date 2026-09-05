import { describe, expect, it, vi } from 'vitest';
import { invoke } from '@/shared/api/tauri';
import { healthCheck } from './health';

vi.mock('@/shared/api/tauri', () => ({
  invoke: vi.fn(),
}));

describe('healthCheck', () => {
  it('returns the backend health response', async () => {
    vi.mocked(invoke).mockResolvedValueOnce('ok' as never);

    await expect(healthCheck()).resolves.toBe('ok');
    expect(invoke).toHaveBeenCalledExactlyOnceWith('health_check');
  });
});
