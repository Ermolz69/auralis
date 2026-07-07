// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { useTranscript } from './useTranscript';
import { getTranscript } from '../api/transcriptApi';

// Mock the dependencies
vi.mock('../api/transcriptApi', () => ({
  getTranscript: vi.fn(),
}));

vi.mock('@/shared/api/tauri', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

describe('useTranscript', () => {
  beforeEach(() => {
    vi.clearAllMocks();
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
});
