// @vitest-environment jsdom
import { describe, it, expect, vi } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { usePasteYoutubeLink } from './usePasteYoutubeLink';
import { useProjectContext, createProjectFromYoutube } from '@/entities/project';

vi.mock('@/entities/project', () => ({
  createProjectFromYoutube: vi.fn(),
  useProjectContext: vi.fn(),
}));

vi.mock('@/shared/router', () => ({
  useNavigation: () => ({
    setCurrentView: vi.fn(),
  }),
}));

describe('usePasteYoutubeLink', () => {
  it('blocks startProject and returns isBlockedByDeletion when deletingProjectId is active', async () => {
    const mockSetProjectId = vi.fn();
    const mockSetProject = vi.fn();

    vi.mocked(useProjectContext).mockReturnValue({
      projectId: 'p-1',
      setProjectId: mockSetProjectId,
      setProject: mockSetProject,
      deletingProjectId: 'p-2', // active deletion
      beginProjectDeletion: vi.fn(),
      finishProjectDeletion: vi.fn(),
      project: null,
    });

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
    const mockSetProjectId = vi.fn();
    const mockSetProject = vi.fn();

    vi.mocked(useProjectContext).mockReturnValue({
      projectId: 'p-1',
      setProjectId: mockSetProjectId,
      setProject: mockSetProject,
      deletingProjectId: null, // no deletion
      beginProjectDeletion: vi.fn(),
      finishProjectDeletion: vi.fn(),
      project: null,
    });

    vi.mocked(createProjectFromYoutube).mockResolvedValue({
      project: { id: 'p-new', title: 'New YouTube Project' } as any,
      job: { id: 'j-1' } as any,
    });

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
    expect(createProjectFromYoutube).toHaveBeenCalledWith('https://youtube.com/watch?v=123');
    expect(mockSetProjectId).toHaveBeenCalledWith('p-new');
  });
});
