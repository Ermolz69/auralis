// @vitest-environment jsdom
import React, { useEffect, useRef, useState } from 'react';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { AppShell } from './AppShell';
import { ProjectContext, useProjectContext, type Project } from '@/entities/project';
import { NavigationProvider, useNavigation, type View } from '@/shared/router';
import { Toaster, toast } from '@/shared/ui/toast';

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
  children,
}: {
  initialView?: View;
  initialProject?: Project | null;
  children?: React.ReactNode;
} = {}) {
  return render(
    <NavigationProvider>
      <InitialView view={initialView} />
      <ProjectHarness initialProject={initialProject}>
        <AppShell>{children ?? <h1>Content heading</h1>}</AppShell>
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
  const [projectId, setProjectId] = useState<string | null>(initialProject?.id ?? null);
  const [currentProject, setProject] = useState<Project | null>(initialProject);
  const [deletingProjectId, setDeletingProjectId] = useState<string | null>(null);
  const deletingProjectIdRef = useRef<string | null>(null);

  return (
    <ProjectContext.Provider
      value={{
        projectId,
        setProjectId,
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
  const { setProjectId, setProject } = useProjectContext();

  return (
    <button
      type="button"
      onClick={() => {
        setProjectId(null);
        setProject(null);
      }}
    >
      Clear project
    </button>
  );
}
