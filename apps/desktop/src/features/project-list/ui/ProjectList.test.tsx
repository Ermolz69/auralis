// @vitest-environment jsdom
import React, { useState, useRef } from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import type { Mock } from 'vitest';
import { render, screen, fireEvent, cleanup, waitFor, act } from '@testing-library/react';
import { ProjectList } from './ProjectList';
import { deleteProject, listProjects, ProjectContext } from '@/entities/project';
import { useNavigation } from '@/shared/router';
import { toast } from '@/shared/ui/toast';
import type { Project } from '@/entities/project';

vi.mock('@/entities/project', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/entities/project')>();
  return {
    ...actual,
    deleteProject: vi.fn(),
    listProjects: vi.fn(),
  };
});

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

const StatefulProjectProvider = ({
  children,
  initialProjectId = 'p-1',
  initialProject = mockProject,
}: {
  children: React.ReactNode;
  initialProjectId?: string | null;
  initialProject?: Project | null;
}) => {
  const [projectId, setProjectId] = useState<string | null>(initialProjectId);
  const [project, setProject] = useState<Project | null>(initialProject);
  const [deletingProjectId, setDeletingProjectId] = useState<string | null>(null);

  const deletingProjectIdRef = useRef<string | null>(null);

  const beginProjectDeletion = (id: string) => {
    if (deletingProjectIdRef.current !== null) return false;
    deletingProjectIdRef.current = id;
    setDeletingProjectId(id);
    return true;
  };

  const finishProjectDeletion = (id: string) => {
    if (deletingProjectIdRef.current === id) {
      deletingProjectIdRef.current = null;
      setDeletingProjectId(null);
    }
  };

  return (
    <ProjectContext.Provider
      value={{
        projectId,
        setProjectId,
        project,
        setProject,
        deletingProjectId,
        beginProjectDeletion,
        finishProjectDeletion,
        operationGeneration: 0,
        captureToken: () => ({ generation: 0 }),
        validateToken: () => true,
      }}
    >
      {children}
    </ProjectContext.Provider>
  );
};

describe('ProjectList', () => {
  let mockSetCurrentView: Mock;
  let testProjects: Project[];

  beforeEach(() => {
    vi.clearAllMocks();
    cleanup();

    mockSetCurrentView = vi.fn();
    (useNavigation as any).mockReturnValue({
      setCurrentView: mockSetCurrentView,
    });

    testProjects = [mockProject, mockProject2];
    (listProjects as Mock).mockImplementation(async () => {
      return [...testProjects];
    });
    (deleteProject as Mock).mockImplementation(async (id: string) => {
      testProjects = testProjects.filter((p) => p.id !== id);
      return null;
    });
  });

  it('Delete Button and Open Button are siblings', async () => {
    render(
      <StatefulProjectProvider>
        <ProjectList />
      </StatefulProjectProvider>,
    );
    await screen.findByText('Test Project');
    const openBtn = screen.getByRole('button', { name: 'Open Test Project' });
    const deleteBtn = screen.getByRole('button', { name: 'Delete Test Project' });
    expect(openBtn.parentElement).toBe(deleteBtn.parentElement?.parentElement);
  });

  it('Empty title uses Untitled Project', async () => {
    render(
      <StatefulProjectProvider>
        <ProjectList />
      </StatefulProjectProvider>,
    );
    await screen.findByText('Untitled Project');
    expect(screen.getByRole('button', { name: 'Delete Untitled Project' })).not.toBeNull();
  });

  it('cancel confirmation does not call API and returns focus to Delete Button', async () => {
    render(
      <StatefulProjectProvider>
        <ProjectList />
      </StatefulProjectProvider>,
    );
    await screen.findByText('Test Project');

    const deleteBtn = screen.getByRole('button', { name: 'Delete Test Project' });
    deleteBtn.focus();
    fireEvent.click(deleteBtn);

    const cancelBtn = screen.getByRole('button', { name: /cancel/i });
    fireEvent.click(cancelBtn);

    expect(deleteProject).not.toHaveBeenCalled();
    expect(document.activeElement).toBe(deleteBtn);
  });

  it('opening deleting project is blocked', async () => {
    render(
      <StatefulProjectProvider initialProjectId="p-1" initialProject={mockProject}>
        <ProjectList />
      </StatefulProjectProvider>,
    );
    await screen.findByText('Test Project');

    const deleteBtn = screen.getByRole('button', { name: 'Delete Test Project' });
    fireEvent.click(deleteBtn);
    fireEvent.click(screen.getByRole('button', { name: /confirm delete/i }));

    const openBtn = screen.getByRole('button', { name: 'Open Test Project' });
    expect((openBtn as HTMLButtonElement).disabled).toBe(true);
  });

  it('BUSY leaves row, shows warning, allows retry', async () => {
    (deleteProject as any).mockRejectedValue({ code: 'BUSY', message: 'Busy error' });
    render(
      <StatefulProjectProvider>
        <ProjectList />
      </StatefulProjectProvider>,
    );
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
    render(
      <StatefulProjectProvider>
        <ProjectList />
      </StatefulProjectProvider>,
    );
    await screen.findByText('Test Project');

    fireEvent.click(screen.getByRole('button', { name: 'Delete Test Project' }));
    fireEvent.click(screen.getByRole('button', { name: /confirm delete/i }));

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith('Repo error');
    });
    expect(screen.getByText('Test Project')).not.toBeNull();
  });

  it('deleting inactive project does not clear context', async () => {
    let currentContext: any;
    const TestConsumer = () => {
      currentContext = React.useContext(ProjectContext);
      return null;
    };

    render(
      <StatefulProjectProvider initialProjectId="p-1" initialProject={mockProject}>
        <ProjectList />
        <TestConsumer />
      </StatefulProjectProvider>,
    );
    await screen.findByText('Untitled Project');

    fireEvent.click(screen.getByRole('button', { name: 'Delete Untitled Project' }));
    fireEvent.click(screen.getByRole('button', { name: /confirm delete/i }));

    await waitFor(() => {
      expect(deleteProject).toHaveBeenCalledWith('p-2');
    });
    expect(currentContext.projectId).toBe('p-1');
  });

  it('successful deletion of current project clears context and navigates to home', async () => {
    let currentContext: any;
    const TestConsumer = () => {
      currentContext = React.useContext(ProjectContext);
      return null;
    };

    render(
      <StatefulProjectProvider initialProjectId="p-1" initialProject={mockProject}>
        <ProjectList />
        <TestConsumer />
      </StatefulProjectProvider>,
    );
    await screen.findByText('Test Project');

    fireEvent.click(screen.getByRole('button', { name: 'Delete Test Project' }));
    fireEvent.click(screen.getByRole('button', { name: /confirm delete/i }));

    await waitFor(() => {
      expect(deleteProject).toHaveBeenCalledWith('p-1');
    });

    await waitFor(() => {
      expect(currentContext.projectId).toBeNull();
      expect(currentContext.project).toBeNull();
      expect(mockSetCurrentView).toHaveBeenCalledWith('home');
      expect(screen.queryByText('Test Project')).toBeNull();
    });
  });

  it('NotFound handled as idempotent success: clears context, redirects, shows success toast', async () => {
    (deleteProject as Mock).mockImplementation(async (id: string) => {
      testProjects = testProjects.filter((p) => p.id !== id);
      throw { code: 'NOT_FOUND', message: 'Project not found' };
    });
    let currentContext: any;
    const TestConsumer = () => {
      currentContext = React.useContext(ProjectContext);
      return null;
    };

    render(
      <StatefulProjectProvider initialProjectId="p-1" initialProject={mockProject}>
        <ProjectList />
        <TestConsumer />
      </StatefulProjectProvider>,
    );
    await screen.findByText('Test Project');

    fireEvent.click(screen.getByRole('button', { name: 'Delete Test Project' }));
    fireEvent.click(screen.getByRole('button', { name: /confirm delete/i }));

    await waitFor(() => {
      expect(deleteProject).toHaveBeenCalledWith('p-1');
    });

    await waitFor(() => {
      expect(currentContext.projectId).toBeNull();
      expect(currentContext.project).toBeNull();
      expect(mockSetCurrentView).toHaveBeenCalledWith('home');
      expect(toast.success).toHaveBeenCalledWith('Project was already removed');
      expect(screen.queryByText('Test Project')).toBeNull();
    });
  });

  it('ignores project-updated event for deleting project but allows for others', async () => {
    let eventCallback: any;
    const { listen } = await import('@/shared/api/tauri');
    (listen as Mock).mockImplementation((event: string, cb: any) => {
      if (event === 'project-updated') eventCallback = cb;
      return Promise.resolve(() => {});
    });

    let resolveDelete: (val: any) => void = () => {};
    const deletePromise = new Promise((resolve) => {
      resolveDelete = resolve;
    });
    (deleteProject as Mock).mockReturnValueOnce(deletePromise);

    render(
      <StatefulProjectProvider initialProjectId="p-1" initialProject={mockProject}>
        <ProjectList />
      </StatefulProjectProvider>,
    );

    await screen.findByText('Test Project');

    // Start delete
    fireEvent.click(screen.getByRole('button', { name: 'Delete Test Project' }));
    fireEvent.click(screen.getByRole('button', { name: /confirm delete/i }));

    // Wait for deletion lock to be active
    await screen.findByRole('button', { name: 'Open Test Project' });
    expect((screen.getByRole('button', { name: 'Open Test Project' }) as HTMLButtonElement).disabled).toBe(true);

    (listProjects as Mock).mockClear();

    // Fire project-updated for the deleting project 'p-1'
    await act(async () => {
      await eventCallback({ payload: { projectId: 'p-1' } });
    });
    expect(listProjects).not.toHaveBeenCalled();

    // Fire project-updated for another project 'p-2'
    await act(async () => {
      await eventCallback({ payload: { projectId: 'p-2' } });
    });
    expect(listProjects).toHaveBeenCalled();

    // Clean up
    await act(async () => {
      resolveDelete(null);
    });
  });

  it('storage error (REPOSITORY) leaves row and returns focus to delete button', async () => {
    (deleteProject as Mock).mockRejectedValue({ code: 'REPOSITORY', message: 'Storage error' });
    render(
      <StatefulProjectProvider>
        <ProjectList />
      </StatefulProjectProvider>,
    );

    await screen.findByText('Test Project');

    const deleteBtn = screen.getByRole('button', { name: 'Delete Test Project' });
    deleteBtn.focus();
    fireEvent.click(deleteBtn);
    fireEvent.click(screen.getByRole('button', { name: /confirm delete/i }));

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith('Storage error');
    });

    expect(screen.getByText('Test Project')).not.toBeNull();
    expect(document.activeElement).toBe(deleteBtn);
  });

  it('keyboard confirmation: cancel has type button, confirm has type submit, form submit works', async () => {
    render(
      <StatefulProjectProvider>
        <ProjectList />
      </StatefulProjectProvider>,
    );

    await screen.findByText('Test Project');
    fireEvent.click(screen.getByRole('button', { name: 'Delete Test Project' }));

    const cancelBtn = screen.getByRole('button', { name: /cancel/i });
    const confirmBtn = screen.getByRole('button', { name: /confirm delete/i });

    expect(cancelBtn.getAttribute('type')).toBe('button');
    expect(confirmBtn.getAttribute('type')).toBe('submit');

    const form = screen.getByTestId('delete-project-form');
    fireEvent.submit(form);

    await waitFor(() => {
      expect(deleteProject).toHaveBeenCalledWith('p-1');
    });
  });

  it('focuses next project open button when deleting a middle project', async () => {
    render(
      <StatefulProjectProvider>
        <ProjectList />
      </StatefulProjectProvider>,
    );

    await screen.findByText('Test Project');
    await screen.findByText('Untitled Project');

    fireEvent.click(screen.getByRole('button', { name: 'Delete Test Project' }));
    fireEvent.click(screen.getByRole('button', { name: /confirm delete/i }));

    await waitFor(() => {
      expect(screen.queryByText('Test Project')).toBeNull();
    });

    const nextOpenBtn = screen.getByRole('button', { name: 'Open Untitled Project' });
    expect(document.activeElement).toBe(nextOpenBtn);
  });

  it('focuses heading when deleting the last remaining project', async () => {
    render(
      <StatefulProjectProvider>
        <ProjectList />
      </StatefulProjectProvider>,
    );

    await screen.findByText('Test Project');
    await screen.findByText('Untitled Project');

    // First delete p-2 (Untitled Project)
    fireEvent.click(screen.getByRole('button', { name: 'Delete Untitled Project' }));
    fireEvent.click(screen.getByRole('button', { name: /confirm delete/i }));

    await waitFor(() => {
      expect(screen.queryByText('Untitled Project')).toBeNull();
    });

    // Now delete p-1 (Test Project)
    fireEvent.click(screen.getByRole('button', { name: 'Delete Test Project' }));
    fireEvent.click(screen.getByRole('button', { name: /confirm delete/i }));

    await waitFor(() => {
      expect(screen.queryByText('Test Project')).toBeNull();
    });

    const heading = screen.getByRole('heading', { name: /recent projects/i });
    expect(document.activeElement).toBe(heading);
  });

  it('focuses new last open button when deleting the last project in a multi-item list', async () => {
    render(
      <StatefulProjectProvider>
        <ProjectList />
      </StatefulProjectProvider>,
    );

    await screen.findByText('Untitled Project');

    fireEvent.click(screen.getByRole('button', { name: 'Delete Untitled Project' }));
    fireEvent.click(screen.getByRole('button', { name: /confirm delete/i }));

    await waitFor(() => {
      expect(screen.queryByText('Untitled Project')).toBeNull();
    });

    const prevOpenBtn = screen.getByRole('button', { name: 'Open Test Project' });
    expect(document.activeElement).toBe(prevOpenBtn);
  });

  it('late response with ghost project does not overwrite projects list', async () => {
    let eventCallback: any;
    const { listen } = await import('@/shared/api/tauri');
    (listen as Mock).mockImplementation((event: string, cb: any) => {
      if (event === 'project-updated') eventCallback = cb;
      return Promise.resolve(() => {});
    });

    let resolveStaleList: (val: Project[]) => void = () => {};
    const staleListPromise = new Promise<Project[]>((resolve) => {
      resolveStaleList = resolve;
    });

    (listProjects as Mock)
      .mockResolvedValueOnce([mockProject, mockProject2]) // 1. Initial mount load
      .mockReturnValueOnce(staleListPromise) // 2. Triggered by event, hangs
      .mockResolvedValueOnce([mockProject2]); // 3. Deletion success refetch resolves immediately

    render(
      <StatefulProjectProvider>
        <ProjectList />
      </StatefulProjectProvider>,
    );

    // Wait for initial load
    await screen.findByText('Test Project');

    // Trigger event for 'p-2' (which is not the deleting project, so it triggers fetch projects - the 2nd call, which hangs)
    await act(async () => {
      await eventCallback({ payload: { projectId: 'p-2' } });
    });

    // Start delete of p-1
    fireEvent.click(screen.getByRole('button', { name: 'Delete Test Project' }));
    fireEvent.click(screen.getByRole('button', { name: /confirm delete/i }));

    // Wait for delete to complete and trigger canonical refetch (the 3rd call) which returns [mockProject2]
    await waitFor(() => {
      expect(screen.queryByText('Test Project')).toBeNull();
    });

    // Now resolve the late listProjects (Gen 2) which still had both projects
    await act(async () => {
      resolveStaleList([mockProject, mockProject2]);
    });

    // Verify that the ghost project did not reappear
    expect(screen.queryByText('Test Project')).toBeNull();
    expect(screen.getByText('Untitled Project')).not.toBeNull();
  });
});
