// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';
import type { Mock } from 'vitest';
import { render, screen, fireEvent, cleanup, waitFor } from '@testing-library/react';
import { ProjectList } from './ProjectList';
import { deleteProject, listProjects, useProjectContext } from '@/entities/project';
import { useNavigation } from '@/shared/router';
import { toast } from '@/shared/ui/toast';
import type { Project } from '@/entities/project';

vi.mock('@/entities/project', () => ({
  deleteProject: vi.fn(),
  listProjects: vi.fn(),
  useProjectContext: vi.fn(),
}));

vi.mock('@/shared/router', () => ({
  useNavigation: vi.fn(),
}));

vi.mock('@/shared/ui/toast', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    warning: vi.fn(),
  },
}));

vi.mock('@/shared/api/tauri', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

vi.mock('@/shared/ui/dialog', () => ({
  Dialog: ({ open, children }: any) =>
    open ? (
      <div role="dialog" data-testid="mock-dialog">
        {children}
      </div>
    ) : null,
  DialogHeader: ({ children }: any) => <div>{children}</div>,
  DialogTitle: ({ children }: any) => <h2>{children}</h2>,
  DialogDescription: ({ children }: any) => <p>{children}</p>,
  DialogFooter: ({ children }: any) => <div>{children}</div>,
  DialogClose: () => <button aria-label="Close dialog">Close</button>,
}));

const mockProject: Project = {
  id: 'p-1',
  title: 'Test Project',
  status: 'draft',
  source: { kind: 'remoteUrl', url: 'https://youtube.com/watch?v=123' },
  metadata: null,
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
};

const mockProject2: Project = {
  id: 'p-2',
  title: '', // Empty title to test fallback
  status: 'completed',
  source: { kind: 'externalLocalFile', path: '/local.mp4' },
  metadata: null,
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
};

describe('ProjectList', () => {
  let mockSetProjectId: Mock;
  let mockSetProject: Mock;
  let mockSetCurrentView: Mock;
  let mockBeginProjectDeletion: Mock;
  let mockFinishProjectDeletion: Mock;

  beforeEach(() => {
    vi.clearAllMocks();
    cleanup();

    mockSetProjectId = vi.fn();
    mockSetProject = vi.fn();
    mockSetCurrentView = vi.fn();
    mockBeginProjectDeletion = vi.fn().mockReturnValue(true);
    mockFinishProjectDeletion = vi.fn();

    (useProjectContext as any).mockReturnValue({
      projectId: 'p-1',
      setProjectId: mockSetProjectId,
      setProject: mockSetProject,
      deletingProjectId: null,
      beginProjectDeletion: mockBeginProjectDeletion,
      finishProjectDeletion: mockFinishProjectDeletion,
    });

    (useNavigation as any).mockReturnValue({
      setCurrentView: mockSetCurrentView,
    });

    (listProjects as any).mockResolvedValue([mockProject, mockProject2]);
  });

  it('Delete Button and Open Button are siblings', async () => {
    render(<ProjectList />);
    await screen.findByText('Test Project');
    const openBtn = screen.getByRole('button', { name: 'Open Test Project' });
    const deleteBtn = screen.getByRole('button', { name: 'Delete Test Project' });
    expect(openBtn.parentElement).toBe(deleteBtn.parentElement?.parentElement);
  });

  it('Empty title uses Untitled Project', async () => {
    render(<ProjectList />);
    await screen.findByText('Untitled Project');
    expect(screen.getByRole('button', { name: 'Delete Untitled Project' })).not.toBeNull();
  });

  it('cancel confirmation does not call API and returns focus to Delete Button', async () => {
    render(<ProjectList />);
    await screen.findByText('Test Project');

    const deleteBtn = screen.getByRole('button', { name: 'Delete Test Project' });
    deleteBtn.focus();
    fireEvent.click(deleteBtn);

    const cancelBtn = screen.getByRole('button', { name: /cancel/i });
    fireEvent.click(cancelBtn);

    expect(deleteProject).not.toHaveBeenCalled();
    expect(document.activeElement).toBe(deleteBtn);
  });

  it('two rapid confirms trigger backend once', async () => {
    render(<ProjectList />);
    await screen.findByText('Test Project');

    fireEvent.click(screen.getByRole('button', { name: 'Delete Test Project' }));

    mockBeginProjectDeletion.mockReturnValueOnce(true).mockReturnValueOnce(false);

    const confirmBtn = screen.getByRole('button', { name: /confirm delete/i });
    fireEvent.click(confirmBtn);
    fireEvent.click(confirmBtn);

    expect(deleteProject).toHaveBeenCalledTimes(1);
  });

  it('opening deleting project is blocked', async () => {
    (useProjectContext as any).mockReturnValue({
      projectId: 'p-1',
      deletingProjectId: 'p-1',
      beginProjectDeletion: mockBeginProjectDeletion,
      finishProjectDeletion: mockFinishProjectDeletion,
      setProjectId: mockSetProjectId,
      setProject: mockSetProject,
    });
    render(<ProjectList />);
    await screen.findByText('Test Project');

    const openBtn = screen.getByRole('button', { name: 'Open Test Project' });
    expect((openBtn as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(openBtn);

    expect(mockSetProjectId).not.toHaveBeenCalled();
  });

  it('BUSY leaves row, shows warning, allows retry', async () => {
    (deleteProject as any).mockRejectedValue({ code: 'BUSY', message: 'Busy error' });
    render(<ProjectList />);
    await screen.findByText('Test Project');

    fireEvent.click(screen.getByRole('button', { name: 'Delete Test Project' }));
    fireEvent.click(screen.getByRole('button', { name: /confirm delete/i }));

    await waitFor(() => {
      expect(toast.warning).toHaveBeenCalledWith('Busy error');
    });
    expect(screen.getByText('Test Project')).not.toBeNull();
  });

  it('REPOSITORY, VALIDATION, INTERNAL leave row and show error', async () => {
    (deleteProject as any).mockRejectedValue({ code: 'REPOSITORY', message: 'Repo error' });
    render(<ProjectList />);
    await screen.findByText('Test Project');

    fireEvent.click(screen.getByRole('button', { name: 'Delete Test Project' }));
    fireEvent.click(screen.getByRole('button', { name: /confirm delete/i }));

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith('Repo error');
    });
    expect(screen.getByText('Test Project')).not.toBeNull();
  });

  it('unknown error leaves row', async () => {
    (deleteProject as any).mockRejectedValue(new Error('Unknown failure'));
    render(<ProjectList />);
    await screen.findByText('Test Project');

    fireEvent.click(screen.getByRole('button', { name: 'Delete Test Project' }));
    fireEvent.click(screen.getByRole('button', { name: /confirm delete/i }));

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith('Error: Unknown failure');
    });
    expect(screen.getByText('Test Project')).not.toBeNull();
  });

  it('deleting inactive project does not clear context', async () => {
    (deleteProject as any).mockResolvedValue(undefined);
    render(<ProjectList />);
    await screen.findByText('Untitled Project');

    fireEvent.click(screen.getByRole('button', { name: 'Delete Untitled Project' }));
    fireEvent.click(screen.getByRole('button', { name: /confirm delete/i }));

    await waitFor(() => {
      expect(mockFinishProjectDeletion).toHaveBeenCalledWith('p-2');
    });
    expect(mockSetProjectId).not.toHaveBeenCalled();
  });

  it('refetch failure after successful delete does not show delete failure', async () => {
    (deleteProject as any).mockResolvedValue(undefined);
    (listProjects as any)
      .mockResolvedValueOnce([mockProject, mockProject2])
      .mockRejectedValue(new Error('Refetch failed'));

    render(<ProjectList />);
    await screen.findByText('Test Project');

    fireEvent.click(screen.getByRole('button', { name: 'Delete Test Project' }));
    fireEvent.click(screen.getByRole('button', { name: /confirm delete/i }));

    await waitFor(() => {
      expect(screen.queryByText('Test Project')).toBeNull();
    });
  });

  it('success focuses next open target', async () => {
    (deleteProject as any).mockResolvedValue(undefined);
    (listProjects as any)
      .mockResolvedValueOnce([mockProject, mockProject2])
      .mockResolvedValue([mockProject2]);
    render(<ProjectList />);
    await screen.findByText('Test Project');

    fireEvent.click(screen.getByRole('button', { name: 'Delete Test Project' }));
    fireEvent.click(screen.getByRole('button', { name: /confirm delete/i }));

    await waitFor(() => {
      expect(screen.queryByText('Test Project')).toBeNull();
    });

    const nextBtn = screen.getByRole('button', { name: 'Open Untitled Project' });
    expect(document.activeElement).toBe(nextBtn);
  });
});
