// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';
import type { Mock } from 'vitest';
import { render, screen, waitFor, fireEvent, cleanup } from '@testing-library/react';
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
  },
}));

vi.mock('@/shared/api/tauri', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

vi.mock('@/shared/ui/dialog', () => ({
  Dialog: ({ open, children }: any) => (open ? <div role="dialog" data-testid="mock-dialog">{children}</div> : null),
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
  title: 'Another Project',
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

  beforeEach(() => {
    vi.clearAllMocks();
    cleanup();
    
    mockSetProjectId = vi.fn();
    mockSetProject = vi.fn();
    mockSetCurrentView = vi.fn();

    (useProjectContext as any).mockReturnValue({
      projectId: 'p-1',
      setProjectId: mockSetProjectId,
      setProject: mockSetProject,
    });

    (useNavigation as any).mockReturnValue({
      setCurrentView: mockSetCurrentView,
    });

    (listProjects as any).mockResolvedValue([mockProject, mockProject2]);
  });

  it('cancel confirmation does not call API', async () => {
    render(<ProjectList />);

    await screen.findByTitle('Test Project');

    const deleteBtn = screen.getByLabelText('Delete Test Project');
    fireEvent.click(deleteBtn);

    const cancelBtn = screen.getByRole('button', { name: /cancel/i });
    fireEvent.click(cancelBtn);

    expect(deleteProject).not.toHaveBeenCalled();
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('delete calls API with correct ID and row is removed after success', async () => {
    (deleteProject as any).mockResolvedValue();
    
    render(<ProjectList />);
    await screen.findByTitle('Test Project');

    const deleteBtn = screen.getByLabelText('Delete Test Project');
    fireEvent.click(deleteBtn);

    const confirmBtn = screen.getByRole('button', { name: /confirm delete/i });
    fireEvent.click(confirmBtn);

    expect(deleteProject).toHaveBeenCalledWith('p-1');
    
    // Project should be removed
    await waitFor(() => {
      expect(screen.queryByTitle('Test Project')).toBeNull();
    });
    expect(screen.queryByTitle('Another Project')).not.toBeNull();
    
    // Context is cleared for selected project
    expect(mockSetProjectId).toHaveBeenCalledWith(null);
    expect(mockSetProject).toHaveBeenCalledWith(null);
    expect(mockSetCurrentView).toHaveBeenCalledWith('home');
    expect(toast.success).toHaveBeenCalledWith('Project deleted successfully');
  });

  it('context is cleared only for the selected Project', async () => {
    (deleteProject as any).mockResolvedValue();
    
    render(<ProjectList />);
    await screen.findByTitle('Another Project');

    // Delete a project that is NOT the current one ('p-2')
    const deleteBtn = screen.getByLabelText('Delete Another Project');
    fireEvent.click(deleteBtn);

    const confirmBtn = screen.getByRole('button', { name: /confirm delete/i });
    fireEvent.click(confirmBtn);

    expect(deleteProject).toHaveBeenCalledWith('p-2');
    
    await waitFor(() => {
      expect(screen.queryByTitle('Another Project')).toBeNull();
    });
    
    // Context should NOT be cleared
    expect(mockSetProjectId).not.toHaveBeenCalled();
    expect(mockSetProject).not.toHaveBeenCalled();
    expect(mockSetCurrentView).not.toHaveBeenCalled();
  });

  it('row remains after error and toast is shown', async () => {
    (deleteProject as any).mockRejectedValue(new Error('Backend failure'));
    
    render(<ProjectList />);
    await screen.findByTitle('Test Project');

    const deleteBtn = screen.getByLabelText('Delete Test Project');
    fireEvent.click(deleteBtn);

    const confirmBtn = screen.getByRole('button', { name: /confirm delete/i });
    fireEvent.click(confirmBtn);

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith('Error: Backend failure');
    });

    // Row should still be there
    expect(screen.queryByTitle('Test Project')).not.toBeNull();
  });

  it('clicking delete does not open Project', async () => {
    render(<ProjectList />);
    await screen.findByTitle('Test Project');

    const deleteBtn = screen.getByLabelText('Delete Test Project');
    fireEvent.click(deleteBtn);

    // It should just open the dialog, not trigger handleOpenProject
    expect(screen.queryByRole('dialog')).not.toBeNull();
    expect(mockSetProjectId).not.toHaveBeenCalledWith('p-1');
    expect(mockSetCurrentView).not.toHaveBeenCalledWith('project');
  });

  it('repeated request during pending is blocked', async () => {
    let resolveDelete: any;
    const deletePromise = new Promise((resolve) => { resolveDelete = resolve; });
    (deleteProject as any).mockReturnValue(deletePromise);
    
    render(<ProjectList />);
    await screen.findByTitle('Test Project');

    const deleteBtn1 = screen.getByLabelText('Delete Test Project');
    fireEvent.click(deleteBtn1);
    const confirmBtn = screen.getByRole('button', { name: /confirm delete/i });
    fireEvent.click(confirmBtn);

    // While deleting p-1, the delete button for p-2 should be disabled
    const deleteBtn2 = screen.getByLabelText('Delete Another Project') as HTMLButtonElement;
    expect(deleteBtn2.disabled).toBe(true);

    // Resolve the promise
    resolveDelete();
    
    await waitFor(() => {
      expect(toast.success).toHaveBeenCalled();
    });
    
    // After success, deletingId is cleared, so remaining buttons are enabled again
    expect(deleteBtn2.disabled).toBe(false);
  });
});
