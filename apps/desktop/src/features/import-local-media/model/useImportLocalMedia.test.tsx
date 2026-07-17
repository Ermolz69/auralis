// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useImportLocalMedia } from './useImportLocalMedia';
import { open } from '@tauri-apps/plugin-dialog';
import { useProjectContext, createProject } from '@/entities/project';

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
}));

vi.mock('@/entities/project', () => ({
  useProjectContext: vi.fn(),
  createProject: vi.fn(),
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
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('blocks handleImport and returns isBlockedByDeletion when deletingProjectId is active initially', async () => {
    vi.mocked(useProjectContext).mockReturnValue({
      projectId: null,
      setProjectId: vi.fn(),
      setProject: vi.fn(),
      deletingProjectId: 'p-1', // Deletion active
      beginProjectDeletion: vi.fn(),
      finishProjectDeletion: vi.fn(),
      project: null,
    });

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

    // Context wrapper to simulate state changing dynamically
    const mockContext: any = {
      projectId: null,
      setProjectId: vi.fn(),
      setProject: vi.fn(),
      deletingProjectId: null, // Initially no deletion
      beginProjectDeletion: vi.fn(),
      finishProjectDeletion: vi.fn(),
      project: null,
    };
    vi.mocked(useProjectContext).mockReturnValue(mockContext);

    const { result, rerender } = renderHook(() => useImportLocalMedia());

    expect(result.current.isBlockedByDeletion).toBe(false);

    // Call handleImport - it will suspend on native file dialog promise
    let importPromise: Promise<void> | undefined;
    act(() => {
      importPromise = result.current.handleImport();
    });

    expect(open).toHaveBeenCalledTimes(1);

    // Simulate project deletion starting while dialog is open
    mockContext.deletingProjectId = 'p-1';
    rerender(); // Trigger re-render to update the ref inside hook

    // Resolve the file picker
    await act(async () => {
      resolveFilePicker('C:\\path\\video.mp4');
      await importPromise;
    });

    // verify that createProject was NOT called because deletingProjectId became active
    expect(createProject).not.toHaveBeenCalled();
  });
});
