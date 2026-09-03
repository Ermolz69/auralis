// @vitest-environment jsdom
import { act, cleanup, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  getLegacyProjectAvatar,
  getProjectAvatar,
  removeLegacyProjectAvatar,
  setProjectAvatar,
} from '@/entities/project';
import { toast } from '@/shared/ui/toast';
import { useProjectAvatar } from './useProjectAvatar';

vi.mock('@/entities/project', () => ({
  getLegacyProjectAvatar: vi.fn(),
  getProjectAvatar: vi.fn(),
  removeLegacyProjectAvatar: vi.fn(),
  setProjectAvatar: vi.fn(),
}));
vi.mock('@/shared/ui/toast', () => ({ toast: { warning: vi.fn() } }));

beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(getProjectAvatar).mockResolvedValue({ dataUrl: 'saved', initialized: true });
  vi.mocked(removeLegacyProjectAvatar).mockReturnValue({ persisted: true });
});
afterEach(cleanup);

describe('durable project avatars', () => {
  it('conditionally migrates legacy data and removes it only after persistence succeeds', async () => {
    vi.mocked(getProjectAvatar).mockResolvedValue({ dataUrl: null, initialized: false });
    vi.mocked(getLegacyProjectAvatar).mockReturnValue('legacy');
    let resolve!: (value: { dataUrl: string; initialized: boolean }) => void;
    vi.mocked(setProjectAvatar).mockReturnValue(
      new Promise((done) => {
        resolve = done;
      }),
    );
    const { result } = renderHook(() => useProjectAvatar('p', false));
    await waitFor(() => expect(setProjectAvatar).toHaveBeenCalledWith('p', 'legacy', true));
    expect(removeLegacyProjectAvatar).not.toHaveBeenCalled();
    await act(async () => resolve({ dataUrl: 'concurrently-saved', initialized: true }));
    expect(result.current.avatar).toBe('concurrently-saved');
    expect(removeLegacyProjectAvatar).toHaveBeenCalledWith('p');
  });

  it('keeps the legacy copy when migration fails', async () => {
    vi.mocked(getProjectAvatar).mockResolvedValue({ dataUrl: null, initialized: false });
    vi.mocked(getLegacyProjectAvatar).mockReturnValue('legacy');
    vi.mocked(setProjectAvatar).mockRejectedValue(new Error('database unavailable'));
    renderHook(() => useProjectAvatar('p', false));
    await waitFor(() => expect(toast.warning).toHaveBeenCalled());
    expect(removeLegacyProjectAvatar).not.toHaveBeenCalled();
  });

  it('never resurrects an explicitly removed avatar from legacy storage', async () => {
    vi.mocked(getProjectAvatar).mockResolvedValue({ dataUrl: null, initialized: true });
    vi.mocked(getLegacyProjectAvatar).mockReturnValue('legacy');
    vi.mocked(removeLegacyProjectAvatar).mockReturnValue({ persisted: false });
    const { result } = renderHook(() => useProjectAvatar('p', false));
    await waitFor(() => expect(removeLegacyProjectAvatar).toHaveBeenCalledWith('p'));
    expect(setProjectAvatar).not.toHaveBeenCalled();
    expect(result.current.avatar).toBeNull();
    expect(toast.warning).toHaveBeenCalledWith(
      'Avatar saved, but its old local copy could not be removed.',
    );
  });

  it('ignores old project loads after switching projects', async () => {
    let resolve!: (value: { dataUrl: string; initialized: boolean }) => void;
    vi.mocked(getProjectAvatar).mockReturnValueOnce(
      new Promise((done) => {
        resolve = done;
      }),
    );
    const { result, rerender } = renderHook(({ id }) => useProjectAvatar(id, false), {
      initialProps: { id: 'old' },
    });
    rerender({ id: 'new' });
    await waitFor(() => expect(result.current.avatar).toBe('saved'));
    await act(async () => resolve({ dataUrl: 'stale', initialized: true }));
    expect(result.current.avatar).toBe('saved');
    expect(getLegacyProjectAvatar).not.toHaveBeenCalledWith('old');
  });

  it.each([
    new File(['<svg/>'], 'avatar.svg', { type: 'image/svg+xml' }),
    new File([new Uint8Array(1024 * 1024 + 1)], 'large.png', { type: 'image/png' }),
  ])(
    'rejects unsupported or oversized uploads without replacing the saved avatar',
    async (file) => {
      const { result } = renderHook(() => useProjectAvatar('p', false));
      await waitFor(() => expect(result.current.avatar).toBe('saved'));
      await act(async () => result.current.updateAvatar(file));
      expect(setProjectAvatar).not.toHaveBeenCalled();
      expect(result.current.avatar).toBe('saved');
      expect(result.current.isSaving).toBe(false);
      expect(toast.warning).toHaveBeenCalled();
    },
  );

  it('saves uploads through the backend without writing preferences', async () => {
    vi.mocked(setProjectAvatar).mockImplementation(async (_id, dataUrl) => ({
      dataUrl,
      initialized: true,
    }));
    const { result } = renderHook(() => useProjectAvatar('p', false));
    await waitFor(() => expect(result.current.avatar).toBe('saved'));
    await act(async () =>
      result.current.updateAvatar(new File(['image'], 'avatar.png', { type: 'image/png' })),
    );
    expect(setProjectAvatar).toHaveBeenCalledWith('p', 'data:image/png;base64,aW1hZ2U=');
    expect(removeLegacyProjectAvatar).not.toHaveBeenCalled();
    expect(result.current.avatar).toBe('data:image/png;base64,aW1hZ2U=');
  });

  it('blocks avatar changes while deleting', async () => {
    const { result } = renderHook(() => useProjectAvatar('p', true));
    await waitFor(() => expect(result.current.avatar).toBe('saved'));
    await act(async () => result.current.updateAvatar(null));
    expect(setProjectAvatar).not.toHaveBeenCalled();
  });
});
