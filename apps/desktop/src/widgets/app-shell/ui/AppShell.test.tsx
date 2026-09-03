// @vitest-environment jsdom
import React, { useEffect, useRef, useState } from 'react';
import { describe, expect, it, vi, beforeEach, type Mock } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { AppShell } from './AppShell';
import {
  listProjects,
  ProjectContext,
  updateProjectPreferences,
  useProjectContext,
  type Project,
} from '@/entities/project';
import { NavigationProvider, useNavigation, type View } from '@/shared/router';
import { Toaster, toast } from '@/shared/ui/toast';
import { JobContext, type JobDto, type JobStoreState } from '@/entities/job';

vi.mock('@/entities/project', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/entities/project')>();
  return { ...actual, listProjects: vi.fn() };
});

const project: Project = {
  id: 'p-1',
  title: 'A very long project title that should remain in the workspace page title',
  status: 'draft',
  source: { kind: 'youtubeUrl', url: 'https://youtube.com/watch?v=123' },
  metadata: null,
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
};

function renderShell({
  initialView = 'home',
  initialProject = project,
  jobState = { phase: 'ready', jobs: {}, buffer: [], pendingRefetch: false, generation: 0 },
  children,
}: {
  initialView?: View;
  initialProject?: Project | null;
  jobState?: JobStoreState | null;
  children?: React.ReactNode;
} = {}) {
  return render(
    <NavigationProvider>
      <InitialView view={initialView} />
      <ProjectHarness initialProject={initialProject}>
        <JobContext.Provider value={jobState}>
          <AppShell jobQueue={<p>Queue contents</p>}>
            {children ?? <h1>Content heading</h1>}
          </AppShell>
        </JobContext.Provider>
        <Toaster />
      </ProjectHarness>
    </NavigationProvider>,
  );
}

function InitialView({ view }: { view: View }) {
  const { setCurrentView } = useNavigation();

  useEffect(() => {
    setCurrentView(view);
  }, [setCurrentView, view]);

  return null;
}

function ProjectHarness({
  children,
  initialProject,
}: {
  children: React.ReactNode;
  initialProject: Project | null;
}) {
  const [currentProject, setProject] = useState<Project | null>(initialProject);
  const projectId = currentProject?.id ?? null;
  const [deletingProjectId, setDeletingProjectId] = useState<string | null>(null);
  const deletingProjectIdRef = useRef<string | null>(null);

  return (
    <ProjectContext.Provider
      value={{
        selection: currentProject
          ? { status: 'open', project: currentProject }
          : { status: 'closed' },
        projectId,
        project: currentProject,
        setProject,
        deletingProjectId,
        beginProjectDeletion: (id) => {
          if (deletingProjectIdRef.current) return false;
          deletingProjectIdRef.current = id;
          setDeletingProjectId(id);
          return true;
        },
        finishProjectDeletion: (id) => {
          if (deletingProjectIdRef.current === id) {
            deletingProjectIdRef.current = null;
            setDeletingProjectId(null);
          }
        },
        operationGeneration: 0,
        captureToken: () => ({ generation: 0, projectId }),
        validateToken: () => true,
      }}
    >
      {children}
    </ProjectContext.Provider>
  );
}

describe('AppShell', () => {
  beforeEach(() => {
    cleanup();
    vi.clearAllMocks();
    localStorage.clear();
    (listProjects as Mock).mockResolvedValue([]);
  });

  it('keeps unrelated jobs in the global counter without marking the subtitle step as running', () => {
    const base: JobDto = {
      id: 'unrelated',
      kind: 'export',
      projectId: project.id,
      revision: 1,
      title: 'Export',
      status: 'running',
      stage: null,
      error: null,
      progress: {
        percent: 50,
        message: '',
        currentStep: null,
        processedItems: null,
        totalItems: null,
      },
      createdAt: project.createdAt,
      updatedAt: project.updatedAt,
    };
    renderShell({
      initialView: 'project',
      jobState: {
        phase: 'ready',
        buffer: [],
        pendingRefetch: false,
        generation: 1,
        jobs: {
          unrelated: base,
          foreign: { ...base, id: 'foreign', projectId: 'another-project', kind: 'dubbing' },
          subtitles: { ...base, id: 'subtitles', kind: 'dubbing', status: 'completed' },
        },
      },
    });
    const step = screen.getByRole('button', { name: /Субтитры/ });
    expect(step.querySelector('.bg-success')).toBeTruthy();
    expect(step.querySelector('.animate-pulse')).toBeNull();
    expect(screen.getByRole('button', { name: /Очередь/ }).textContent).toContain('2');
  });

  it('marks the current destination and disables workspace without a project', async () => {
    renderShell({ initialProject: null });

    const projects = screen.getByRole('button', { name: 'Projects' });
    const workspace = screen.getByRole('button', {
      name: 'Workspace unavailable without an active project',
    });

    await waitFor(() => {
      expect(projects.getAttribute('aria-current')).toBe('page');
    });
    expect((workspace as HTMLButtonElement).disabled).toBe(true);
  });

  it('supports Home to Project to Home navigation and focuses main content', async () => {
    renderShell();

    fireEvent.click(screen.getByRole('button', { name: 'Workspace' }));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'Workspace' }).getAttribute('aria-current')).toBe(
        'page',
      );
      expect(document.activeElement).toBe(screen.getByRole('heading', { name: 'Content heading' }));
    });

    fireEvent.click(screen.getByRole('button', { name: 'Projects' }));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'Projects' }).getAttribute('aria-current')).toBe(
        'page',
      );
      expect(document.activeElement).toBe(screen.getByRole('heading', { name: 'Content heading' }));
    });
  });

  it('returns from settings to the previous project context', async () => {
    renderShell({ initialView: 'project' });

    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'Workspace' }).getAttribute('aria-current')).toBe(
        'page',
      );
    });

    fireEvent.click(screen.getByRole('button', { name: 'Settings' }));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'Settings' }).getAttribute('aria-current')).toBe(
        'page',
      );
    });

    fireEvent.click(screen.getByRole('button', { name: 'Back' }));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'Workspace' }).getAttribute('aria-current')).toBe(
        'page',
      );
    });
  });

  it('returns from settings to projects when the active project disappears', async () => {
    renderShell({
      initialView: 'project',
      children: (
        <>
          <h1>Content heading</h1>
          <ClearProjectButton />
        </>
      ),
    });

    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'Workspace' }).getAttribute('aria-current')).toBe(
        'page',
      );
    });

    fireEvent.click(screen.getByRole('button', { name: 'Settings' }));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'Settings' }).getAttribute('aria-current')).toBe(
        'page',
      );
    });

    fireEvent.click(screen.getByRole('button', { name: 'Clear project' }));
    fireEvent.click(screen.getByRole('button', { name: 'Back' }));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'Projects' }).getAttribute('aria-current')).toBe(
        'page',
      );
      expect(
        (
          screen.getByRole('button', {
            name: 'Workspace unavailable without an active project',
          }) as HTMLButtonElement
        ).disabled,
      ).toBe(true);
    });
  });

  it('renders global toast notifications once', async () => {
    renderShell();

    toast.success('Project deleted');

    await screen.findByText('Project deleted');
    expect(screen.getAllByText('Project deleted')).toHaveLength(1);
    expect(document.querySelectorAll('[aria-live="polite"]')).toHaveLength(1);
  });

  it('requires a project name before creation', async () => {
    renderShell({ initialProject: null });

    fireEvent.click(screen.getByRole('button', { name: 'Create project' }));

    const message = await screen.findByText('Укажите название проекта');
    const input = screen.getByRole('textbox', { name: 'Project name' });
    expect(input.getAttribute('aria-invalid')).toBe('true');
    expect(document.activeElement).toBe(input);

    const alert = message.closest('[role="alert"]');
    expect(alert).not.toBeNull();
    fireEvent.click(within(alert as HTMLElement).getByRole('button', { name: 'Close toast' }));
    await waitFor(() => expect(screen.queryByText('Укажите название проекта')).toBeNull());
  });

  it('resizes the project and pipeline panes with the keyboard', () => {
    renderShell();

    const separator = screen.getByRole('separator', {
      name: 'Resize projects and pipeline panels',
    });
    const sidebarBody = separator.parentElement!;
    Object.defineProperty(sidebarBody, 'clientHeight', { configurable: true, value: 400 });
    expect(separator.getAttribute('aria-valuenow')).toBe('190');

    fireEvent.keyDown(separator, { key: 'ArrowDown' });

    expect(separator.getAttribute('aria-valuenow')).toBe('202');

    fireEvent.keyDown(separator, { key: 'ArrowUp' });
    expect(separator.getAttribute('aria-valuenow')).toBe('190');

    for (let i = 0; i < 10; i++) {
      fireEvent.keyDown(separator, { key: 'ArrowDown' });
    }
    expect(separator.getAttribute('aria-valuenow')).toBe('280');

    Object.defineProperty(sidebarBody, 'clientHeight', { configurable: true, value: 350 });
    fireEvent.keyDown(separator, { key: 'ArrowDown' });
    expect(separator.getAttribute('aria-valuenow')).toBe('230');

    for (let i = 0; i < 10; i++) {
      fireEvent.keyDown(separator, { key: 'ArrowUp' });
    }
    expect(separator.getAttribute('aria-valuenow')).toBe('120');

    fireEvent.keyDown(separator, { key: 'ArrowDown' });
    expect(separator.getAttribute('aria-valuenow')).toBe('132');
  });

  it('closes the job queue with Escape and restores focus to its trigger', async () => {
    renderShell();

    const trigger = screen.getByRole('button', { name: 'Очередь' });
    fireEvent.click(trigger);
    expect(screen.getByText('Queue contents')).not.toBeNull();

    fireEvent.keyDown(window, { key: 'Escape' });

    await waitFor(() => {
      expect(screen.queryByText('Queue contents')).toBeNull();
      expect(document.activeElement).toBe(trigger);
    });
  });

  it('refreshes pinned projects after preferences change', async () => {
    (listProjects as Mock).mockResolvedValue([project]);
    renderShell();

    await screen.findByText('Нет закреплённых проектов');
    updateProjectPreferences(project.id, { pinned: true });

    await waitFor(() => {
      expect(screen.getByRole('button', { name: project.title })).not.toBeNull();
    });

    updateProjectPreferences(project.id, { pinned: false });

    await waitFor(() => {
      expect(screen.queryByRole('button', { name: project.title })).toBeNull();
    });
  });

  it('dismisses global toast without leaving focus on a removed control', async () => {
    renderShell();

    toast.error('Pipeline failed');

    await screen.findByText('Pipeline failed');

    const toastAlert = screen.getByRole('alert');
    const closeToast = within(toastAlert).getByRole('button', { name: 'Close toast' });
    closeToast.focus();
    fireEvent.click(closeToast);

    await waitFor(() => {
      expect(screen.queryByText('Pipeline failed')).toBeNull();
    });

    expect(document.activeElement).not.toBe(closeToast);
    expect(document.querySelectorAll('[aria-live="polite"]')).toHaveLength(1);
  });
});

function ClearProjectButton() {
  const { setProject } = useProjectContext();

  return (
    <button
      type="button"
      onClick={() => {
        setProject(null);
      }}
    >
      Clear project
    </button>
  );
}
