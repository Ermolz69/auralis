import type { Meta, StoryObj } from '@storybook/react-vite';
import React, { useEffect, useRef, useState } from 'react';
import { AppShell } from './AppShell';
import { ProjectContext, type Project } from '@/entities/project';
import { NavigationProvider, useNavigation, type View } from '@/shared/router';

const longProject: Project = {
  id: 'p-1',
  title:
    'Quarterly product launch keynote with a very long source title that must not crowd navigation',
  status: 'processing',
  source: { kind: 'youtubeUrl', url: 'https://youtube.com/watch?v=123' },
  metadata: null,
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
};

const meta = {
  title: 'Widgets/AppShell',
  component: AppShell,
  parameters: {
    layout: 'fullscreen',
  },
  tags: ['autodocs'],
} satisfies Meta<typeof AppShell>;

export default meta;
type Story = StoryObj<typeof meta>;

export const WithoutActiveProject: Story = {
  render: () => (
    <ShellStory initialView="home" project={null}>
      <div className="p-8">
        <h1 className="text-3xl font-bold">Projects</h1>
      </div>
    </ShellStory>
  ),
};

export const WithLongProjectTitle: Story = {
  render: () => (
    <ShellStory initialView="project" project={longProject}>
      <div className="p-8">
        <h1 className="text-3xl font-bold">Workspace</h1>
      </div>
    </ShellStory>
  ),
};

export const SettingsWithProjectReturn: Story = {
  render: () => (
    <ShellStory initialView="settings" project={longProject}>
      <div className="p-8">
        <h1 className="text-3xl font-bold">Settings</h1>
      </div>
    </ShellStory>
  ),
};

function ShellStory({
  children,
  initialView,
  project,
}: {
  children: React.ReactNode;
  initialView: View;
  project: Project | null;
}) {
  return (
    <NavigationProvider>
      <InitialView view={initialView} />
      <ProjectStoryProvider project={project}>
        <AppShell>{children}</AppShell>
      </ProjectStoryProvider>
    </NavigationProvider>
  );
}

function InitialView({ view }: { view: View }) {
  const { setCurrentView } = useNavigation();

  useEffect(() => {
    setCurrentView(view);
  }, [setCurrentView, view]);

  return null;
}

function ProjectStoryProvider({
  children,
  project,
}: {
  children: React.ReactNode;
  project: Project | null;
}) {
  const [currentProject, setProject] = useState<Project | null>(project);
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
