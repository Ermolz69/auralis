// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { act, renderHook, waitFor } from '@testing-library/react';
import { useTranscript } from './useTranscript';
import { getTranscript } from '../api/transcriptApi';
import { listen } from '@/shared/api/tauri';
import type { Transcript } from './types';

// Mock the dependencies
vi.mock('../api/transcriptApi', () => ({
  getTranscript: vi.fn(),
}));

vi.mock('@/shared/api/tauri', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

function deferred<T>() {
  let resolve: (value: T) => void = () => undefined;
  let reject: (reason: unknown) => void = () => undefined;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

describe('useTranscript', () => {
  beforeEach(() => {
    vi.mocked(getTranscript).mockReset();
    vi.mocked(listen)
      .mockReset()
      .mockResolvedValue(() => undefined);
  });

  it('fetches transcript and updates state if projectId matches', async () => {
    (getTranscript as any).mockResolvedValueOnce({
      language: 'en',
      segments: [],
    });

    const { result } = renderHook(() => useTranscript('project-1'));

    expect(result.current.isLoading).toBe(true);

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.transcript).toEqual({ language: 'en', segments: [] });
  });

  it('ignores response if projectId has changed during fetch', async () => {
    let resolveFirstFetch: any;
    const firstFetchPromise = new Promise((resolve) => {
      resolveFirstFetch = resolve;
    });

    let resolveSecondFetch: any;
    const secondFetchPromise = new Promise((resolve) => {
      resolveSecondFetch = resolve;
    });

    (getTranscript as any).mockImplementation((id: string) => {
      if (id === 'project-1') return firstFetchPromise;
      if (id === 'project-2') return secondFetchPromise;
      return Promise.resolve(null);
    });

    const { result, rerender } = renderHook(({ id }: { id: string | null }) => useTranscript(id), {
      initialProps: { id: 'project-1' },
    });

    // Rerender with a new project ID before the first fetch completes
    rerender({ id: 'project-2' });

    // Resolve the first fetch now
    resolveFirstFetch({ language: 'en', segments: [] });

    // Wait a tick to let the promise resolve
    await new Promise((r) => setTimeout(r, 0));

    // The transcript should NOT be updated because the project ID changed
    expect(result.current.transcript).toBeNull();

    // Resolve the second fetch
    resolveSecondFetch({ language: 'fr', segments: [] });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    // Now it should be updated
    expect(result.current.transcript).toEqual({ language: 'fr', segments: [] });
  });

  it('refetches the current project when transcript-ready arrives', async () => {
    let transcriptReadyHandler: any = null;
    vi.mocked(listen).mockImplementation(async (_eventName, handler: any) => {
      transcriptReadyHandler = handler;
      return () => undefined;
    });
    vi.mocked(getTranscript)
      .mockResolvedValueOnce(null)
      .mockResolvedValueOnce({
        language: 'en',
        segments: [
          {
            id: 'segment-1',
            index: 0,
            startMs: 0,
            endMs: 1000,
            sourceText: 'Ready subtitle',
          },
        ],
      });

    const { result } = renderHook(() => useTranscript('project-1'));

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    transcriptReadyHandler?.({ payload: { projectId: 'project-1', jobId: 'job-1' } });

    await waitFor(() => {
      expect(result.current.transcript?.segments[0]?.sourceText).toBe('Ready subtitle');
    });
    expect(getTranscript).toHaveBeenCalledTimes(2);
  });

  it('refetch action uses the current project id', async () => {
    vi.mocked(getTranscript)
      .mockResolvedValueOnce(null)
      .mockResolvedValueOnce({ language: 'en', segments: [] });

    const { result } = renderHook(() => useTranscript('project-1'));

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    result.current.refetch();

    await waitFor(() => {
      expect(getTranscript).toHaveBeenCalledTimes(2);
    });
    expect(getTranscript).toHaveBeenLastCalledWith('project-1');
  });

  it('does not let an older same-project response overwrite a newer refetch', async () => {
    let resolveInitial: (value: unknown) => void = () => undefined;
    const initial = new Promise((resolve) => {
      resolveInitial = resolve;
    });
    vi.mocked(getTranscript)
      .mockReturnValueOnce(initial as ReturnType<typeof getTranscript>)
      .mockResolvedValueOnce({ language: 'en', segments: [] });

    const { result } = renderHook(() => useTranscript('project-1'));
    await act(async () => {
      await result.current.refetch();
    });
    expect(result.current.transcript).toEqual({ language: 'en', segments: [] });

    await act(async () => {
      resolveInitial({ language: 'stale', segments: [] });
      await initial;
    });
    expect(result.current.transcript).toEqual({ language: 'en', segments: [] });
  });

  it('keeps the newest scope when navigation returns from A through B to A', async () => {
    const firstA = deferred<Transcript | null>();
    const projectB = deferred<Transcript | null>();
    const secondA = deferred<Transcript | null>();
    vi.mocked(getTranscript)
      .mockReturnValueOnce(firstA.promise)
      .mockReturnValueOnce(projectB.promise)
      .mockReturnValueOnce(secondA.promise);

    const { result, rerender } = renderHook(({ id }: { id: string | null }) => useTranscript(id), {
      initialProps: { id: 'project-a' as string | null },
    });
    rerender({ id: 'project-b' });
    rerender({ id: 'project-a' });

    await act(async () => {
      secondA.resolve({ language: 'current-a', segments: [] });
      await secondA.promise;
    });
    expect(result.current.transcript?.language).toBe('current-a');

    await act(async () => {
      firstA.resolve({ language: 'stale-a', segments: [] });
      projectB.resolve({ language: 'stale-b', segments: [] });
      await Promise.all([firstA.promise, projectB.promise]);
    });

    expect(result.current.transcript?.language).toBe('current-a');
    expect(result.current.isLoading).toBe(false);
    expect(result.current.error).toBeNull();
  });

  it('does not let a stale error or finally finish a newer same-project request', async () => {
    const initial = deferred<Transcript | null>();
    const refetch = deferred<Transcript | null>();
    vi.mocked(getTranscript)
      .mockReturnValueOnce(initial.promise)
      .mockReturnValueOnce(refetch.promise);

    const { result } = renderHook(() => useTranscript('project-1'));
    act(() => {
      void result.current.refetch();
    });

    await act(async () => {
      initial.reject(new Error('stale failure'));
      await initial.promise.catch(() => undefined);
    });

    expect(result.current.error).toBeNull();
    expect(result.current.isLoading).toBe(true);

    await act(async () => {
      refetch.resolve({ language: 'current', segments: [] });
      await refetch.promise;
    });

    expect(result.current.transcript?.language).toBe('current');
    expect(result.current.error).toBeNull();
    expect(result.current.isLoading).toBe(false);
  });

  it('masks the previous scope immediately and resets state when the scope closes', async () => {
    vi.mocked(getTranscript).mockResolvedValueOnce({ language: 'project-a', segments: [] });
    const projectB = deferred<Transcript | null>();
    vi.mocked(getTranscript).mockReturnValueOnce(projectB.promise);

    const { result, rerender } = renderHook(({ id }: { id: string | null }) => useTranscript(id), {
      initialProps: { id: 'project-a' as string | null },
    });
    await waitFor(() => {
      expect(result.current.transcript?.language).toBe('project-a');
    });

    rerender({ id: 'project-b' });
    expect(result.current.transcript).toBeNull();
    expect(result.current.isLoading).toBe(true);
    expect(result.current.error).toBeNull();

    rerender({ id: null });
    expect(result.current.transcript).toBeNull();
    expect(result.current.isLoading).toBe(false);
    expect(result.current.error).toBeNull();

    await act(async () => {
      projectB.resolve({ language: 'stale-b', segments: [] });
      await projectB.promise;
    });
    expect(result.current.transcript).toBeNull();
    expect(result.current.isLoading).toBe(false);
  });

  it('invalidates a pending request and releases a listener that resolves after unmount', async () => {
    const request = deferred<Transcript | null>();
    const listener = deferred<() => void>();
    const unlisten = vi.fn();
    vi.mocked(getTranscript).mockReturnValueOnce(request.promise);
    vi.mocked(listen).mockReturnValueOnce(listener.promise);

    const { unmount } = renderHook(() => useTranscript('project-1'));
    unmount();

    await act(async () => {
      request.resolve({ language: 'stale', segments: [] });
      listener.resolve(unlisten);
      await Promise.all([request.promise, listener.promise]);
    });

    expect(unlisten).toHaveBeenCalledOnce();
  });
});
