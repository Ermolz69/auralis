// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useImportLocalMedia } from './useImportLocalMedia';
import { open } from '@tauri-apps/plugin-dialog';
import { useProjectContext, createProject, deleteProject } from '@/entities/project';
import { importLocalMedia } from '@/entities/media';

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
}));

vi.mock('@/entities/project', () => ({
  useProjectContext: vi.fn(),
  createProject: vi.fn(),
  deleteProject: vi.fn(),
}));

vi.mock('@/entities/media', () => ({
  importLocalMedia: vi.fn(),
}));

vi.mock('@/shared/router', () => ({
  useNavigation: () => ({
    setCurrentView: vi.fn(),
  }),
}));

describe('useImportLocalMedia', () => {
  let mockContextValue: any;

  beforeEach(() => {
    vi.clearAllMocks();
    mockContextValue = {
      projectId: 'p-1',
      setProjectId: vi.fn(),
      setProject: vi.fn(),
      deletingProjectId: null,
      beginProjectDeletion: vi.fn(),
      finishProjectDeletion: vi.fn(),
      operationGeneration: 1,
      captureToken: () => ({ generation: mockContextValue.operationGeneration }),
      validateToken: (token: any) =>
        mockContextValue.deletingProjectId === null &&
        token.generation === mockContextValue.operationGeneration,
    };
    vi.mocked(useProjectContext).mockReturnValue(mockContextValue);
  });

  it('blocks handleImport and returns isBlockedByDeletion when deletingProjectId is active initially', async () => {
    mockContextValue.deletingProjectId = 'p-2';

    const { result } = renderHook(() => useImportLocalMedia());

    expect(result.current.isBlockedByDeletion).toBe(true);

    await act(async () => {
      await result.current.handleImport();
    });

    expect(open).not.toHaveBeenCalled();
    expect(createProject).not.toHaveBeenCalled();
  });

  it('allows start but cancels import if deletion starts while file picker is open', async () => {
    let resolveFilePicker: (val: any) => void = () => {};
    const filePickerPromise = new Promise((resolve) => {
      resolveFilePicker = resolve;
    });
    vi.mocked(open).mockReturnValue(filePickerPromise as any);

    const { result, rerender } = renderHook(() => useImportLocalMedia());

    expect(result.current.isBlockedByDeletion).toBe(false);

    let importPromise: Promise<void> | undefined;
    act(() => {
      importPromise = result.current.handleImport();
    });

    expect(open).toHaveBeenCalledTimes(1);

    // Simulate delete begin
    act(() => {
      mockContextValue.operationGeneration += 1;
      mockContextValue.deletingProjectId = 'p-1';
    });
    rerender();

    // Resolve file picker
    await act(async () => {
      resolveFilePicker('C:\\path\\video.mp4');
      await importPromise;
    });

    expect(createProject).not.toHaveBeenCalled();
  });

  it('blocks concurrent second request inside the same generation (during picker)', async () => {
    let resolveFilePicker: (val: any) => void = () => {};
    const filePickerPromise = new Promise((resolve) => {
      resolveFilePicker = resolve;
    });
    vi.mocked(open).mockReturnValue(filePickerPromise as any);

    const { result } = renderHook(() => useImportLocalMedia());

    let firstPromise: Promise<void> | undefined;
    act(() => {
      firstPromise = result.current.handleImport();
    });

    expect(open).toHaveBeenCalledTimes(1);

    // Try starting a second import flow concurrently
    let secondRes: any;
    await act(async () => {
      secondRes = await result.current.handleImport();
    });

    expect(secondRes).toBeUndefined(); // blocked by active lock

    await act(async () => {
      resolveFilePicker(null);
      await firstPromise;
    });
  });

  it('verifies picker cancel releases activeAttempt lock and clears loading', async () => {
    vi.mocked(open).mockResolvedValue(null);

    const { result } = renderHook(() => useImportLocalMedia());

    await act(async () => {
      await result.current.handleImport();
    });

    expect(result.current.isImporting).toBe(false);
    expect(createProject).not.toHaveBeenCalled();

    // Should allow import again
    vi.mocked(open).mockResolvedValue('C:\\path\\video.mp4');
    vi.mocked(createProject).mockResolvedValue({ id: 'p-new' } as any);
    vi.mocked(importLocalMedia).mockResolvedValue({ id: 'p-new' } as any);

    await act(async () => {
      await result.current.handleImport();
    });
    expect(createProject).toHaveBeenCalled();
  });

  it('verifies that no compensating delete request is issued and the successfully created backend project response is discarded by the stale frontend flow', async () => {
    vi.mocked(open).mockResolvedValue('C:\\path\\video.mp4');

    let resolveCreate: (val: any) => void = () => {};
    const createPromise = new Promise((resolve) => {
      resolveCreate = resolve;
    });
    vi.mocked(createProject).mockReturnValue(createPromise as any);

    const { result, rerender } = renderHook(() => useImportLocalMedia());

    let importPromise: Promise<void> | undefined;
    act(() => {
      importPromise = result.current.handleImport();
    });

    // Invalidate token before createProject resolves
    act(() => {
      mockContextValue.operationGeneration += 1;
    });
    rerender();

    await act(async () => {
      resolveCreate({ id: 'p-new', title: 'New Video' } as any);
      await importPromise;
    });

    // Verify: importLocalMedia not called, deleteProject not called (no compensating delete), context unchanged
    expect(importLocalMedia).not.toHaveBeenCalled();
    expect(deleteProject).not.toHaveBeenCalled();
    expect(mockContextValue.setProjectId).not.toHaveBeenCalled();
  });

  it('stale local REPOSITORY error does not set the error state', async () => {
    vi.mocked(open).mockResolvedValue('C:\\path\\video.mp4');
    vi.mocked(createProject).mockResolvedValue({ id: 'p-new' } as any);

    let rejectImport: (reason: any) => void = () => {};
    const importPromise = new Promise((_, reject) => {
      rejectImport = reject;
    });
    importPromise.catch(() => {});
    vi.mocked(importLocalMedia).mockReturnValue(importPromise as any);

    const { result, rerender } = renderHook(() => useImportLocalMedia());

    let handlePromise: Promise<void> | undefined;
    act(() => {
      handlePromise = result.current.handleImport();
    });

    // Invalidate token
    act(() => {
      mockContextValue.operationGeneration += 1;
    });
    rerender();

    await act(async () => {
      rejectImport(new Error('Repository Error'));
      await handlePromise;
    });

    expect(result.current.error).toBeNull();
  });

  it('protects against raw leakage in local media import errors', async () => {
    vi.mocked(open).mockResolvedValue('C:\\path\\video.mp4');
    vi.mocked(createProject).mockResolvedValue({ id: 'p-new' } as any);

    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    vi.mocked(importLocalMedia).mockRejectedValue(
      new Error('C:\\Users\\secret\\video.mp4 token=SECRET'),
    );

    const { result } = renderHook(() => useImportLocalMedia());

    await act(async () => {
      await result.current.handleImport();
    });

    expect(result.current.error).toBe('An unexpected system error occurred');

    consoleErrorSpy.mock.calls.forEach((call) => {
      const logStr = JSON.stringify(call);
      expect(logStr).not.toContain('secret');
      expect(logStr).not.toContain('SECRET');
      expect(logStr).not.toContain('token');
      expect(logStr).not.toContain('video.mp4');
    });

    consoleErrorSpy.mockRestore();
  });
});
