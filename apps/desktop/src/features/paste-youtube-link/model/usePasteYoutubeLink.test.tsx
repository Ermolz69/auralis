// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { usePasteYoutubeLink } from './usePasteYoutubeLink';
import { useProjectContext, createProjectFromYoutube } from '@/entities/project';

vi.mock('@/entities/project', () => ({
  createProjectFromYoutube: vi.fn(),
  useProjectContext: vi.fn(),
}));
const mockSetCurrentView = vi.fn();
const mockSetPipelineStep = vi.fn();

vi.mock('@/shared/router', () => ({
  useNavigation: () => ({
    setCurrentView: mockSetCurrentView,
    setPipelineStep: mockSetPipelineStep,
  }),
}));

describe('usePasteYoutubeLink', () => {
  let mockContextValue: any;
  beforeEach(() => {
    vi.clearAllMocks();
    mockContextValue = {
      projectId: 'p-1',
      setProject: vi.fn(),
      deletingProjectId: null,
      beginProjectDeletion: vi.fn(),
      finishProjectDeletion: vi.fn(),
      operationGeneration: 1,
      captureToken: () => ({
        generation: mockContextValue.operationGeneration,
        projectId: mockContextValue.projectId,
      }),
      validateToken: (token: any) =>
        mockContextValue.deletingProjectId === null &&
        token.generation === mockContextValue.operationGeneration &&
        token.projectId === mockContextValue.projectId,
    };
    vi.mocked(useProjectContext).mockReturnValue(mockContextValue);
  });

  it('exposes download states and retries the same source after failure', async () => {
    const { result } = renderHook(() => usePasteYoutubeLink());
    expect(result.current.status).toBe('Idle');
    act(() => result.current.setUrl('https://youtube.com/watch?v=retry'));
    let rejectDownload!: (error: Error) => void;
    vi.mocked(createProjectFromYoutube).mockImplementationOnce(
      () =>
        new Promise((_, reject) => {
          rejectDownload = reject;
        }),
    );
    let pending!: ReturnType<typeof result.current.startProject>;
    act(() => {
      pending = result.current.startProject();
    });
    expect(result.current.status).toBe('Downloading');
    await act(async () => {
      rejectDownload(new Error('Network unavailable'));
      await pending;
    });
    expect(result.current.status).toBe('DownloadFailed');
    expect(result.current.isStarting).toBe(false);
    expect(result.current.url).toBe('https://youtube.com/watch?v=retry');
    expect(mockContextValue.setProject).not.toHaveBeenCalled();
    vi.mocked(createProjectFromYoutube).mockResolvedValueOnce({ id: 'p-1', title: 'Retry' } as any);
    await act(async () => {
      await result.current.startProject();
    });
    expect(result.current.status).toBe('Ready');
    expect(result.current.error).toBeNull();
    expect(createProjectFromYoutube).toHaveBeenNthCalledWith(
      2,
      'https://youtube.com/watch?v=retry',
      'p-1',
    );
  });

  it('blocks startProject when deletingProjectId is active', async () => {
    mockContextValue.deletingProjectId = 'p-2';
    const { result } = renderHook(() => usePasteYoutubeLink());
    expect(result.current.isBlockedByDeletion).toBe(true);
    act(() => {
      result.current.setUrl('https://youtube.com/watch?v=123');
    });
    let res;
    await act(async () => {
      res = await result.current.startProject();
    });
    expect(res).toBeNull();
    expect(createProjectFromYoutube).not.toHaveBeenCalled();
  });

  it('allows startProject when deletingProjectId is null', async () => {
    vi.mocked(createProjectFromYoutube).mockResolvedValue({
      id: 'p-new',
      title: 'New',
    } as any);
    const { result } = renderHook(() => usePasteYoutubeLink());
    expect(result.current.isBlockedByDeletion).toBe(false);
    act(() => {
      result.current.setUrl('https://youtube.com/watch?v=123');
    });
    let res;
    await act(async () => {
      res = await result.current.startProject();
    });
    expect(res).not.toBeNull();
    expect(createProjectFromYoutube).toHaveBeenCalledWith('https://youtube.com/watch?v=123', 'p-1');
    expect(mockContextValue.setProject).toHaveBeenCalledWith({ id: 'p-new', title: 'New' });
    expect(mockSetPipelineStep).toHaveBeenCalledWith('source');
    expect(mockSetCurrentView).toHaveBeenCalledWith('project');
    expect(result.current.isStarting).toBe(false);
  });

  it('fences out response when delete occurs during create request', async () => {
    let resolveCreate: (val: any) => void = () => {};
    const createPromise = new Promise((resolve) => {
      resolveCreate = resolve;
    });
    vi.mocked(createProjectFromYoutube).mockReturnValue(createPromise as any);
    const { result, rerender } = renderHook(() => usePasteYoutubeLink());
    act(() => {
      result.current.setUrl('https://youtube.com/watch?v=123');
    });
    let startPromise: Promise<any>;
    act(() => {
      startPromise = result.current.startProject();
    });
    expect(result.current.isStarting).toBe(true);
    act(() => {
      mockContextValue.operationGeneration += 1;
      mockContextValue.deletingProjectId = 'p-1';
    });
    rerender();
    await act(async () => {
      resolveCreate({ id: 'p-new' } as any);
      await startPromise;
    });
    expect(result.current.url).toBe('https://youtube.com/watch?v=123');
    expect(mockContextValue.setProject).not.toHaveBeenCalled();
    expect(result.current.isStarting).toBe(false);
  });

  it('fences out response when switch occurs during create request', async () => {
    let resolveCreate: (val: any) => void = () => {};
    const createPromise = new Promise((resolve) => {
      resolveCreate = resolve;
    });
    vi.mocked(createProjectFromYoutube).mockReturnValue(createPromise as any);
    const { result, rerender } = renderHook(() => usePasteYoutubeLink());
    act(() => {
      result.current.setUrl('https://youtube.com/watch?v=123');
    });
    let startPromise: Promise<any>;
    act(() => {
      startPromise = result.current.startProject();
    });
    act(() => {
      mockContextValue.projectId = 'p-other';
      mockContextValue.operationGeneration += 1;
    });
    rerender();
    await act(async () => {
      resolveCreate({ id: 'p-new' } as any);
      await startPromise;
    });
    expect(result.current.url).toBe('https://youtube.com/watch?v=123');
    expect(mockContextValue.setProject).not.toHaveBeenCalled();
    expect(result.current.isStarting).toBe(false);
  });

  it('stale YouTube NOT_FOUND error does not set the state/error', async () => {
    let rejectCreate: (reason: any) => void = () => {};
    const createPromise = new Promise((_, reject) => {
      rejectCreate = reject;
    });
    createPromise.catch(() => {});
    vi.mocked(createProjectFromYoutube).mockReturnValue(createPromise as any);
    const { result, rerender } = renderHook(() => usePasteYoutubeLink());
    act(() => {
      result.current.setUrl('https://youtube.com/watch?v=123');
    });
    let startPromise: Promise<any>;
    act(() => {
      startPromise = result.current.startProject();
    });
    act(() => {
      mockContextValue.operationGeneration += 1;
    });
    rerender();
    await act(async () => {
      rejectCreate({ code: 'NOT_FOUND', message: 'Project not found' });
      await startPromise;
    });
    expect(result.current.error).toBeNull();
    expect(result.current.isStarting).toBe(false);
  });

  it('blocks concurrent second request inside the same generation', async () => {
    let resolveCreate: (val: any) => void = () => {};
    const createPromise = new Promise((resolve) => {
      resolveCreate = resolve;
    });
    vi.mocked(createProjectFromYoutube).mockReturnValue(createPromise as any);
    const { result } = renderHook(() => usePasteYoutubeLink());
    act(() => {
      result.current.setUrl('https://youtube.com/watch?v=123');
    });
    let firstPromise: Promise<any>;
    act(() => {
      firstPromise = result.current.startProject();
    });
    let secondRes: any;
    await act(async () => {
      secondRes = await result.current.startProject();
    });
    expect(secondRes).toBeNull();
    await act(async () => {
      resolveCreate({ id: 'p-new' } as any);
      await firstPromise;
    });
  });

  it('allows a new request after generation transition and prevents old finally from resetting loading/lock', async () => {
    let resolve1: (val: any) => void = () => {};
    const promise1 = new Promise((resolve) => {
      resolve1 = resolve;
    });
    vi.mocked(createProjectFromYoutube).mockReturnValueOnce(promise1 as any);
    const { result, rerender } = renderHook(() => usePasteYoutubeLink());
    act(() => {
      result.current.setUrl('https://youtube.com/watch?v=123');
    });
    let startPromise1: Promise<any>;
    act(() => {
      startPromise1 = result.current.startProject();
    });
    expect(result.current.isStarting).toBe(true);
    act(() => {
      mockContextValue.operationGeneration += 1;
    });
    rerender();
    let resolve2: (val: any) => void = () => {};
    const promise2 = new Promise((resolve) => {
      resolve2 = resolve;
    });
    vi.mocked(createProjectFromYoutube).mockReturnValueOnce(promise2 as any);
    let startPromise2: Promise<any>;
    act(() => {
      startPromise2 = result.current.startProject();
    });
    expect(result.current.isStarting).toBe(true);
    await act(async () => {
      resolve1({ id: 'p-stale' } as any);
      await startPromise1;
    });
    expect(result.current.isStarting).toBe(true);
    await act(async () => {
      resolve2({ id: 'p-new' } as any);
      await startPromise2;
    });
    expect(result.current.isStarting).toBe(false);
  });

  it('releases activeAttempt lock on actual error', async () => {
    vi.mocked(createProjectFromYoutube).mockRejectedValue(new Error('Failed') as any);
    const { result } = renderHook(() => usePasteYoutubeLink());
    act(() => {
      result.current.setUrl('https://youtube.com/watch?v=123');
    });
    await act(async () => {
      await result.current.startProject();
    });
    expect(result.current.error).toBe('An unexpected system error occurred');
    expect(result.current.isStarting).toBe(false);
    vi.mocked(createProjectFromYoutube).mockResolvedValue({
      id: 'p-new',
    } as any);
    let res;
    await act(async () => {
      res = await result.current.startProject();
    });
    expect(res).not.toBeNull();
  });

  it('protects against raw leakage in startProject errors', async () => {
    vi.mocked(createProjectFromYoutube).mockRejectedValue(
      new Error('C:\\Users\\secret\\video.mp4 token=SECRET') as any,
    );
    const { result } = renderHook(() => usePasteYoutubeLink());
    act(() => {
      result.current.setUrl('https://youtube.com/watch?v=123');
    });

    await act(async () => {
      await result.current.startProject();
    });

    expect(result.current.error).toBe('An unexpected system error occurred');
  });
});
